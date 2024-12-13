use color_eyre::eyre::Context;
use color_eyre::Result;
use image::flat::SampleLayout;
use image::{DynamicImage, GenericImageView};
use ndarray::{Array2, ShapeBuilder};
use tracing::info;

fn to_ndarray(image: DynamicImage) -> Result<Array2<f32>> {
    let image = image.to_rgb32f();
    let SampleLayout {
        height,
        height_stride,
        width,
        width_stride,
        ..
    } = image.sample_layout();
    let shape = (height as usize, width as usize);
    let strides = (height_stride, width_stride);
    Array2::from_shape_vec(shape.strides(strides), image.into_raw())
        .wrap_err("Failed to convert image to ndarray")
}

/// Compute the mean Structural Similarity Index between two images.
/// Source: https://github.com/openrecall/openrecall/blob/main/openrecall/app.py#L247
fn similarity_index(image1: &DynamicImage, image2: &DynamicImage, l: f32) -> Result<f32> {
    if image1.dimensions() != image2.dimensions() {
        // if images aren't the same size, consider them completely different
        info!("Images are different sizes, returning 0.0");
        return Ok(0.0);
    }

    let k1 = 0.01;
    let k2 = 0.03;
    let c1 = (k1 * l) * (k1 * l);
    let c2 = (k2 * l) * (k2 * l);

    let image1 = image1.grayscale();
    let image2 = image2.grayscale();

    let image1 = to_ndarray(image1).with_context(|| "Failed to convert image1 to ndarray")?;
    let image2 = to_ndarray(image2).with_context(|| "Failed to convert image2 to ndarray")?;

    let mu1 = image1.mean().expect("must have a mean");
    let mu2 = image2.mean().expect("must have a mean");

    let sigma1_sq = image1.var(1.0);
    let sigma2_sq = image2.var(1.0);
    let sigma12 = ((image1 - mu1) * (image2 - mu2))
        .mean()
        .expect("must have a mean");

    let ssim_index = ((2.0 * mu1 * mu2 + c1) * (2.0 * sigma12 + c2))
        / ((mu1 * mu1 + mu2 * mu2 + c1) * (sigma1_sq + sigma2_sq + c2));

    info!("SSIM index: {}", ssim_index);
    Ok(ssim_index)
}

pub fn is_similar(image1: &DynamicImage, image2: &DynamicImage, threshold: f32) -> Result<bool> {
    if threshold == 0.0 {
        Ok(false)
    } else {
        let similarity = similarity_index(image1, image2, 255.0)?;
        Ok(similarity > threshold)
    }
}
