# Oxeylyzer repl

## Building with cargo
To use the analyzer, clone the repo, make sure you have rust nightly installed, and run `cargo run +nightly --release` in a terminal. This will build and run the project. For future uses you can use this command again, or manually take the `.exe` in the newly created `target/release` folder, drop it in the root folder and execute that. There might be a better way to do this but I'm not sure.

## Using the repl
Type `help` to get all commands with their explanation, and `<command> help` to get a more detailed description about it. Should be pretty self-explanatory :thumbsup:

As an aside for `generate` and `improve`, I run them with `1000` usually but you get pretty good results with 500 usually as well. You can run with more but it might start taking a while.

As a piece of advice however, if you for example have a vowel block in mind you want to use, pinning it and running `improve` can speed up your generation process by a _lot_. For example, if you know you want `eu ao i` (for English) you can pin these positions and run `improve semimak <amount>` (or any other layout with this vowel setup) to get about a 250% speed increase or something similar, just by pinning 5 keys.

## Configuration
There are a lot of metrics that can be configured, which all happens in the `config.toml`. Keys used in generation can be configured as well in `languages_default.cfg`, though I would probably not recommend changing these unless you want to do some custom stuff like pretending `e` is on a thumb key and replacing it with `/`. Dedicated thumb keys will be added some time in the future. 

### Pins
Pins allow you to lock certain keys to a certain position when you run `improve` on a certain layout. if you change a `.` into an `x`, it becomes pinned. This is useful if you want certain keys to be in certain locations, but want to optimize everything else.

### Defaults
`language` is the language the repl starts out in, and `trigram_precision` is the amount of trigrams that are used during generation. Note however that this does not actually work yet, it's hardcoded to be 1000 everywhere. I will fix this at some point.

### Weights
This is where the magic happens.

#### Heatmap
A metric that uses a preset heatmap to make sure high freq keys don't go into very faraway locations, even if it works out everywhere else. If you wouldn't use this, you might get similar indexes to whorf where something that's high freq is placed somewhere off to the side with everything else clustered around it to minimize distance.

#### Fspeed
Short for finger speed, and is basically a weighted sum of sfbs, dsfbs, and some weaker versions of those (up to skipgrams with 3 chars inbetween) _accounting for distance and finger strength_. This is extremely useful because it allows you to more accurately assess how bad certain high speed movement is.

#### Lateral Penalty
A penalty multiplied directly by lateral distance in fspeed. Did not give the results I hoped for so it's 1.0 by default, which is no extra penalty.

#### Dsfb ratio
A ratio which is used to weigh dsfbs and their variants _compared to sfbs_. Because dsfbs are usually around 6% frequency on normal keyboards and sfbs around 1%, the default is 0.11 which comes down to dsfbs being 66% as important as sfbs.

#### Scissors
Scissors are kind of a loosey goosey pattern that refers in essence to adjacent keys jumping up or down 2 rows, e.g. qwerty `u,`, `ex`, `qx` etc. Qwerty `im`, `in` and `ec` (assuming you use angle mod) are excluded from this, while 2 others are added, being qwerty `qs` and `pl`. It's not super precise, but it's very useful for checking your layout doesn't have a lot of very wonky patterns on it.

#### Inrolls and Outrolls
These are defined as trigrams, being 2 keys on one hand into one in the other, or vice versa. The two keys on the same hand cannot be sfbs. Inrolls mean the flow is inward, e.g. `pinky -> middle`, `ring -> index`, whereas outrolls are the opposite. These are generally considered the fastest pattern on a layout.

#### Onehands
Onehands are trigrams on the same hand that all flow in a particular direction, e.g. `pinky -> ring -> middle` or `ring -> middle -> index`. Inconsistent but you generally don't want to punish those.

#### Alternates and Alternates Sfs
Alternation is a trigram where the first and third keys are on the same hand, but not the middle one, e.g. qwerty `ake` or `pen`. Sfs stands for Same Finger Skipgram, and is a special (worse) case of Alternation where you press the 1st and 3rd key with the same finger, which tends to be quite a lot slower.

#### Redirects and Bad Redirects
Redirects are trigrams where you press all three keys with the same hand, but they change direction. Examples include qwerty `ads`, `pul`, `era`. Bad redirects are a special case of these, where none of the keys include index, which makes them worse. Normal redirects are considered okay-ish in some cases, but generally you want to punish redirects at least a little bit, and bad redirects even more.

#### Max Finger Use
This basically exists to be a soft cap on how much %usage you can put on a finger before it's 'too much'. It is useful in columns that do well on paper but have very high total frequency, like `pnb` pinky.

#### Finger Speed
These finger speed weigths determine the strength of certain fingers, and divides the distance used for fspeed accordingly.