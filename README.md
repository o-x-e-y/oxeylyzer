# Oxeylyzer repl

## Building with cargo
to use the analyzer, clone the repo, make sure you have rust nightly installed, and run `cargo run +nightly --release` in a terminal. This will build and run the project. For future uses you can use this command again, or manually take the `.exe` in the newly created `target/release` folder, drop it in the root folder and execute that. There might be a better way to do this but I'm not sure.

## Using the repl
type `help` to get all commands with their explanation, and `<command> help` to get a more detailed description about it. Should be pretty self-explanatory :thumbsup:

as an aside for `generate` and `improve`, I run them with `1000` usually but you get pretty good results with 500 usually as well. You can run with more but it might start taking a while.

Also, if you have a vowel block in mind you want to use, pinning it and running `improve` can speed up your generation process by a _lot_. For example, if you know you want `eu ao i` you can pin these positions and run `improve semimak <amount>` (or any other layout with this vowel setup) to get about a 250% speed increase or something similar, just by pinning 5 keys.