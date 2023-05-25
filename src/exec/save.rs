use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use image::{ImageBuffer, Rgba};
use serde_json::{from_value, Value};

use super::{ExecutionError, GraphContext};

pub fn save_texture(
    meta: &[Value],
    inputs: &[Value],
    ctx: &GraphContext,
) -> Result<Vec<Value>, ExecutionError> {
    let Some(path) = meta.get(0) else {
        return Err(ExecutionError::ValueError);
    };
    let Some(texture) = inputs.get(0) else {
        return Err(ExecutionError::ValueError);
    };

    let path: String = from_value(path.clone())?;
    let texture: String = from_value(texture.clone())?;

    let image_data = STANDARD_NO_PAD.decode(texture).unwrap();

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        ctx.dimensions.0 as u32,
        ctx.dimensions.1 as u32,
        image_data,
    )
    .ok_or(ExecutionError::ValueError)?;

    image.save(path)?;

    Ok(vec![])
}
