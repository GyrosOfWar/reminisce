use image::{flat::SampleLayout, DynamicImage};
use ndarray::{Array2, ShapeBuilder};

fn to_ndarray(image: DynamicImage) -> Array2<f32> {
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
    Array2::from_shape_vec(shape.strides(strides), image.into_raw()).unwrap()
}

/// Compute the mean Structural Similarity Index between two images.
/// Source: https://github.com/openrecall/openrecall/blob/main/openrecall/app.py#L247
fn similarity_index(image1: &DynamicImage, image2: &DynamicImage, l: f32) -> f32 {
    let k1 = 0.01;
    let k2 = 0.03;
    let c1 = (k1 * l) * (k1 * l);
    let c2 = (k2 * l) * (k2 * l);

    let image1 = image1.grayscale();
    let image2 = image2.grayscale();

    let image1 = to_ndarray(image1);
    let image2 = to_ndarray(image2);

    let mu1 = image1.mean().expect("must have a mean");
    let mu2 = image2.mean().expect("must have a mean, too");

    let sigma1_sq = image1.var(1.0);
    let sigma2_sq = image2.var(1.0);
    let sigma12 = ((image1 - mu1) * (image2 - mu2))
        .mean()
        .expect("must have a mean, three");

    let ssim_index = ((2.0 * mu1 * mu2 + c1) * (2.0 * sigma12 + c2))
        / ((mu1 * mu1 + mu2 * mu2 + c1) * (sigma1_sq + sigma2_sq + c2));

    ssim_index
}

pub fn is_similar(image1: &DynamicImage, image2: &DynamicImage) -> bool {
    let similarity = similarity_index(image1, image2, 255.0);
    similarity > 0.9
}
