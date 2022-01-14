## How to build

First time run:

`npm install http-server -g`

cd to this folder, run `./build.sh` or  `./build.sh --features webgl`

## Notice

Do not run build.sh with the cargo check/build/run in the same terminal. This will cause build cache invalidation because the RUSTCFLAG is not as same as rust analyzer.