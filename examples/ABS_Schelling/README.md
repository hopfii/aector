# Schelling Segregation simulation with actors
This example implements the Schelling segregation model using actors as agents. A simple UI using SDL2 is provided.

# Prerequisites
- SDL2 has to be installed in order to be able to run the UI. See https://github.com/Rust-SDL2/rust-sdl2 for further instructions/details.
- Quick install and build of SDL2 under Windows:
```console
cargo install cargo-vcpkg
cargo vcpkg build
cargo build
```

- Quick install and build of SDL2 under Ubuntu:
```console
sudo apt install libsdl2-dev libsdl2-mixer-dev libsdl2-gfx-dev libsdl2-ttf-dev libsdl2-image-dev
cargo build
```


- Run example:
```console
cargo run --example run_sim_with_ui
```

- Change simulation parameters in:
  - src/lib.rs for grid-size
  - examples/run_sim_with_ui.rs for general simulation settings and parameters

