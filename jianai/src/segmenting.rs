use crate::model::*;
use crate::schema::*;
use crate::types;
use anyhow::{anyhow, Result};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::Pool, scoped_futures::ScopedFutureExt, AsyncConnection,
    AsyncPgConnection,
};
use image::GenericImageView;
use ndarray::{s, Array, Axis};
use ort::{CUDAExecutionProvider, Session};
use redis::{
    aio::MultiplexedConnection, AsyncCommands, Client, ExistenceCheck, RedisResult, SetOptions,
};
use redis_pool::RedisPool;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::task::spawn_blocking;
use tracing::info;

#[async_recursion::async_recursion]
pub async fn segmenting_loop(
    redis_pool: RedisPool<Client, MultiplexedConnection>,
    pg_pool: Pool<AsyncPgConnection>,
    image_folder: PathBuf,
    model_path: PathBuf,
) -> Result<()> {
    info!("Preparing segmenting");
    use diesel_async::RunQueryDsl;
    let mut pg_conn = pg_pool.get().await?;
    let untagged_images: Vec<Image> = images::dsl::images
        .filter(images::segmented.eq(false))
        .load(&mut pg_conn)
        .await?;

    let mut redis = redis_pool.aquire().await?;
    let hostname = gethostname::gethostname()
        .into_string()
        .map_err(|e| anyhow!("{e:?}"))?;

    // Wishing I could use `try_find`.
    let mut todo = None;
    let mut mc_key = String::new();
    for i in untagged_images.into_iter() {
        mc_key = format!("segmenting-{}", i.id);
        let x: RedisResult<String> = redis
            .set_options(
                mc_key.clone(),
                hostname.clone(),
                SetOptions::default().conditional_set(ExistenceCheck::NX),
            )
            .await;
        match x {
            Ok(_) => {
                todo = Some(i);
                break;
            }
            Err(e) => {
                if e.is_cluster_error()
                    || e.is_connection_dropped()
                    || e.is_connection_refusal()
                    || e.is_io_error()
                    || e.is_timeout()
                    || e.is_unrecoverable_error()
                {
                    (Err(e))?; // real errors
                } else {
                    continue; // nil/null reply
                }
            }
        }
    }

    if let Some(image) = todo {
        info!("Found image to segment");
        let image1 = Image {
            filename: image_folder
                .join(image.filename.clone())
                .to_str()
                .expect("msg")
                .to_string(),
            ..image.clone()
        };
        let image = Arc::new(image);
        let model_path1 = model_path.clone();
        match spawn_blocking(|| segmenting(image1, model_path1)).await? {
            Ok(segments) => {
                info!("Segmenting done. Updating DB.");
                pg_conn
                    .transaction(|pg_conn| {
                        (async move {
                            let inserts: Vec<_> = segments
                                .iter()
                                .filter(|segment| segment.class == "cat")
                                .map(|segment| {
                                    (
                                        segments::image_id.eq(image.id),
                                        segments::bounding_box.eq(segment.bounding_box),
                                    )
                                })
                                .collect();
                            diesel::insert_into(segments::table)
                                .values(&inserts)
                                .execute(pg_conn)
                                .await?;
                            diesel::update(images::dsl::images.find(image.id))
                                .set(images::segmented.eq(true))
                                .get_result::<Image>(pg_conn)
                                .await?;
                            Ok(()) as Result<(), diesel::result::Error>
                        })
                        .scope_boxed()
                    })
                    .await?;

                redis.del(mc_key).await?;

                drop(pg_conn);
                drop(redis);
                info!("Start next round");
                segmenting_loop(redis_pool, pg_pool, image_folder, model_path).await?;
            }
            e => {
                redis.del(mc_key).await?;
                e?;
            }
        }
    }

    Ok(())
}

fn segmenting(image: Image, model_path: PathBuf) -> Result<Vec<Segment>> {
    let file = fs::read(image.filename.clone())?;
    let image = image::load_from_memory(&file)?;
    let mut input = Array::zeros((1, 3, image.height().try_into()?, image.width().try_into()?));

    // Array: FromIterator only suports one dimensional array.
    for pixel in image.pixels() {
        let x = pixel.0.try_into()?;
        let y = pixel.1.try_into()?;
        let [r, g, b, _] = pixel.2 .0;
        let r: f32 = r.into();
        let g: f32 = g.into();
        let b: f32 = b.into();
        input[[0, 0, y, x]] = r / 255.0;
        input[[0, 1, y, x]] = g / 255.0;
        input[[0, 2, y, x]] = b / 255.0;
    }

    ort::init()
        .with_execution_providers([CUDAExecutionProvider::default().build()])
        .commit()?;
    //  yolo export imgsz='(2048,1536)' model=yolov8x.pt format=onnx
    let session = Session::builder()?.commit_from_file(model_path)?;
    let outputs = session.run(ort::inputs!["images" => input.view()]?)?;
    let output = outputs["output0"]
        .try_extract_tensor::<f32>()?
        .view()
        .t()
        .into_owned();

    let output = output.slice(s![.., .., 0]);

    let mut boxes = output
        .axis_iter(Axis(0))
        .filter_map(|row| {
            // each row contains 84 items. First 4 are bound box, the other 80 are possibilities for each class.
            let (class_id, prob) = row
                .iter()
                // skip bounding box coordinates
                .skip(4)
                .enumerate()
                .max_by_key(|possibility| {
                    ordered_float::NotNan::new(*possibility.1).expect("Will this be NaN?")
                })
                .unwrap();
            let label = YOLOV8_CLASS_LABELS[class_id];
            let xc = *row.get([0_usize]).expect("msg");
            let yc = *row.get([1_usize]).expect("msg");
            let w = *row.get([2_usize]).expect("msg");
            let h = *row.get([3_usize]).expect("msg");
            if *prob < 0.5 {
                None
            } else {
                Some((
                    types::Box {
                        point1: types::Point {
                            x: xc - w / 2.0,
                            y: yc - h / 2.0,
                        },
                        point2: types::Point {
                            x: xc + w / 2.0,
                            y: yc + h / 2.0,
                        },
                    },
                    label,
                    *prob,
                ))
            }
        })
        .collect::<Vec<_>>();

    boxes.sort_by(|box1, box2| box2.2.total_cmp(&box1.2));
    let mut result = Vec::new();

    while !boxes.is_empty() {
        let (a, b, c) = *boxes
            .first_mut()
            .ok_or(anyhow!("Unable to find index 0 in `boxes`"))?;
        result.push(Segment {
            bounding_box: a,
            class: b.to_string(),
            _posibility: c,
        });
        boxes.retain(|&(box1, _, _)| a.intersection_area(&box1) / a.union_area(&box1) < 0.7);
    }

    Ok(result)
}

#[derive(Clone)]
struct Segment {
    bounding_box: types::Box,
    class: String,
    _posibility: f32,
}

const YOLOV8_CLASS_LABELS: [&str; 80] = [
    "person",
    "bicycle",
    "car",
    "motorcycle",
    "airplane",
    "bus",
    "train",
    "truck",
    "boat",
    "traffic light",
    "fire hydrant",
    "stop sign",
    "parking meter",
    "bench",
    "bird",
    "cat",
    "dog",
    "horse",
    "sheep",
    "cow",
    "elephant",
    "bear",
    "zebra",
    "giraffe",
    "backpack",
    "umbrella",
    "handbag",
    "tie",
    "suitcase",
    "frisbee",
    "skis",
    "snowboard",
    "sports ball",
    "kite",
    "baseball bat",
    "baseball glove",
    "skateboard",
    "surfboard",
    "tennis racket",
    "bottle",
    "wine glass",
    "cup",
    "fork",
    "knife",
    "spoon",
    "bowl",
    "banana",
    "apple",
    "sandwich",
    "orange",
    "broccoli",
    "carrot",
    "hot dog",
    "pizza",
    "donut",
    "cake",
    "chair",
    "couch",
    "potted plant",
    "bed",
    "dining table",
    "toilet",
    "tv",
    "laptop",
    "mouse",
    "remote",
    "keyboard",
    "cell phone",
    "microwave",
    "oven",
    "toaster",
    "sink",
    "refrigerator",
    "book",
    "clock",
    "vase",
    "scissors",
    "teddy bear",
    "hair drier",
    "toothbrush",
];
