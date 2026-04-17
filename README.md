

# The real stand alone clearcore flasher

Replaces the hacky [CLI version](https://github.com/paul-fornage/Standalone-ClearCore-BIN-Flasher). 

This one has a GUI, multi platform, and has a language that isn't made by Microsoft!

## Download

[Latest release](https://github.com/paul-fornage/clearcore-flash-rs/releases/latest)

## Why?

Some companies that put rockets in space can not figure out the cli version I made, so to help remote debugging I made this.

It was also a batch file that relied on many different executables, kind of hacky, and windows only because Teknic only released an exe for the flasher in the arduino wrapper.

## Todo

- [ ] clean up upload backed <-> front end communication
- [ ] Work on the bossa wrapper, maybe make that one more rusty and async? very nebulous but like 
should not be declaring a struct and then writing to it using a mut ref parameter in a function the next line to initialize it.
- [ ] Option to enable/disable displaying timestamps.
- [ ] add CLI capability. 


### Raspberry pi
At the time of writing, RPI OS seems to have some issues with their Vulkan driver. 
Works if you run with `WGPU_BACKEND=gl ./clearcore-flash-rs`