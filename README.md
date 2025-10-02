A little toy implementation of the [canny-edge-detection] algorithm in WPGU.

# Usage

```bash
cargo run --release -- <path to image file> <path to output directory>

# Example: Applies edge detection to the given image and saves each change after each step to the given output directory.
cargo run --release -- /tmp/image.png /tmp/output_dir
```

# Example

Here's an example which images will be generated.

## Source image (`example-images/castle.jpg`):

![source image](./example-images/castle.jpg)

## 1. Gray scale

![gray scale](./example-images/1_gray_scale.png)

## 2. Gaussian filter

![gaussian filter](./example-images/2_gaussian.png)

## 3. Horizontal and vertical edges

### 3.1 Horizontal edges

![horizontal edges](./example-images/3_horizontal.png)

### 3.2 Vertical edges

![vertical edges](./example-images/3_vertical.png)

## 4. Magnitude and radians

### 4.1 Magnitude

![magnitude](./example-images/4_magnitude.png)

### 4.2 Radians

![radians](./example-images/4_magnitude.png)

## 5. Non-maximum-suppression

![non-maximum-suppression](./example-images/5_non_maximum_suppression.png)

## 6. Double Threshold

![double threshold](./example-images/6_threshold_texture.png)

## 7. Edge tracking

![edge tracking](./example-images/7_edge_tracking.png)

# Sources to learn

- Canny algo: https://justin-liang.com/tutorials/canny/
- How to create gray scale: https://www.baeldung.com/cs/convert-rgb-to-grayscale

[canny-edge-detection]: https://en.wikipedia.org/wiki/Canny_edge_detector
