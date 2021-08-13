# Copy and Convert DAC_SDC Dataset to [YOLOX](https://github.com/Megvii-BaseDetection/YOLOX) COCO format

## Usage
```shell script
cargo run -- -s <source_dir> -t <target_dir> 
```
- `source_dir` should contain all class folders.

For more information see `cargo run -- -h`.

## Feature

Use [rayon](https://github.com/rayon-rs/rayon) to parse annotations in parallel.
May consume a lot of CPU resources.
