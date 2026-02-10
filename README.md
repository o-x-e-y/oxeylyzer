# Oxeylyzer repl

## Building with cargo
To use the analyzer, clone the repo and make sure you have rust nightly installed. If you have, skip the next step.

To install rust, visit [the official installation page](https://www.rust-lang.org/learn/get-started) and follow the instructions there. When installing, make sure you add rust to PATH. Once you have installed, you may need to restart in order for the command to be recognised. After restarting, you can try running `rustup install nightly` and `rustup default nightly` to make sure the compiler can use all unstable features.

Once you have done this, you can open a terminal in the folder you cloned into, and run `cargo run --release`. This will build and run the project. For future uses you can use this command again, or `cargo install --path ./` from within the root folder of the project, which makes it runnable from anywhere as `oxeylyzer`!

## Using the repl
Type `help` to get all commands with their explanation, and `<command> help` to get a more detailed description about it. Should be pretty self-explanatory :thumbsup:

As an aside for `generate` and `improve`, I run them with `1000` usually but you get pretty good results with 500 usually as well. You can run with more but it might start taking a while.

As a piece of advice however, if you for example have a vowel block in mind you want to use, pinning it and running `improve` can speed up your generation process by a _lot_. For example, if you know you want `eu ao i` (for English) you can pin these positions and run `improve semimak <amount>` (or any other layout with this vowel setup) to get about a 250% speed increase or something similar, just by pinning 5 keys.

## Configuration
There are a lot of metrics that can be configured, which all happens in the `config.toml`. Keys used in generation can be configured as well in `languages_default.cfg`, though I would probably not recommend changing these unless you want to do some custom stuff like pretending `e` is on a thumb key and replacing it with `/`. Dedicated thumb keys will be added some time in the future. 

### Pins
Pins allow you to lock certain keys to a certain position when you run `improve` on a certain layout. if you change a `.` into an `x`, it becomes pinned. This is useful if you want certain keys to be in certain locations, but want to optimize everything else.

### Defaults
`language` is the language the repl starts out in, and `trigram_precision` is the amount of trigrams that are used during generation. Note however that this does not actually work yet, it's hardcoded to be 1000 everywhere. I will fix this at some point. There is also `keyboard_type`, which sets some values for the heatmap the analyzer uses. This has a few settings:

* Ansi - Iso - JIS - Rowstag:

Default fingering, no angle mod. This means lower left is punished quite heavily.

* Iso Angle:

Same as the above, except left bottom now has a lower weight.

* Ansi Angle:

Same as Iso Angle, except it punishes the bottom left pinky a lot to have nothing important go there, and make sure it can easily go to qwerty `b`. Note that this only changes the heatmap, it won't change sfb calculation.

* Ortho:

Punishes certain positions slightly more and others slightly less due to stagger. Homerow weights unchanged. Useful if you have an ortho keyboard.

* Colstag

Punishes some top row positions a bit more than ortho, others a bit less. Useful if you have board with column stagger.

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

## Importing raw text

You can import raw text for creating your own corpus and corpus rules. To do this create a folder in `/static/text/` and place your text within that folder. The name of the folder will be the name used for loading the text in the REPL. For example we create the folder `icelandic` (no capitals allowed) within `/static/text/`, place `icelandic_sentences.txt` within and load the text with `load icelandic`.

## Creating your own corpus rules

You can generate language data files using your own rules now! There are a few settings that you can use for them. As a shortcut, if your corpus is just English, you can create a `.toml` file with a single line: `inherits = ["default"]`. That should cover everything you need.

### Corpus config files

Using the `.toml` files found in the subfolders of `./corpus_config`, you set the exact keys you want to treat differently. When you run `load <language> [--raw]`, the matching corpus config file's rules will be selected. `--raw` means everything barring control characters will be maintained, and is useful if you're not sure on what rules to create yet.

All direct subfolders are searched for this, so you can keep your own rulesets in a different folder to keep things nice and tidy. All characters not specified will be simulated but discarded in the final result. The allowed fields are the following:

#### inherits

This is an array `[]` that contain references to other config files. Most provided configs use `default`, which has a couple of useful formatting features like unshifting latin characters and some punctuation and changing some unconventional quotation marks to the more common appostrophe (which itself is the unshifted version of `"`). One caveat is that this does not check for circular references, so if you have one file `A` that references `B` and have `B` reference `A`, that _will_ get stuck in an infinite loop. Don't do that :thumbsup:

#### letters_to_lowercase

This takes a string of lowercase characters. What this will do is that in the data, it will retain the lowercase letters provided as normal, and store the uppercase variants (if a proper one exists) as ` <char>`. That space is essentially a simulated shift press, where for example on qwerty `mU` would not actually be a true sfb because there's a shift press inbetween `m` and `u`. This does not work for e.g. symbols, which have their own function.

#### punct_unshifted

This takes two strings: `from`, a string which contains the UPPERCASE version of the punct, and `to`, which is a string of equal length what the uppercase punct will be transformed into (also with a simulated shift press like uppercase letters). You can use this to test different combinations of punct if you want, though the default ones are probably fine for most use cases.

#### one_to_one

This basically does what `punct_unshifted` does, except it doesn't add the simulated shift press. This is useful if you have certain characters you'd rather transform into something else. I use this to normalize some uncommon punctuation, but there are probably other usecases for this.

#### to_multiple

This takes a `list` attribute which is an array `[]` of arrays. The arrays inside contain two elements: a single character, and the sequence of keys this character should be converted into. In the `default.toml` config this is used to convert ellipsis into three separate `.` characters, but this is very useful for languages like Spanish or French where you can use an accent key (denoted by `*` in those languages) to collapse all kinds of accented keys into a single key + letter. You don't need to change the corpus for this at all, it's purely handled within these corpus rules.

This also takes an optional argument `uppercase_versions`, which takes a `true/false` value. This is false by default, but when set to true it will also generate uppercase versions of these sequences. For example, if you have an `["ç", "*c"]` rule, you will get `["Ç", " *c"]` completely for free which is useful for these alphabetic conversions.

### languages_default.cfg

In the root there is also a file which contains language names, and the 30 keys that are used for generation by default. You can and should select these yourself (I think it might straight up crash if you try to generate for a language that doesn't have these). Usually a pretty good way to find out good keys is to take the top 30, give or take some punctuation you might not want.

### Coming up with good rules

Having made rules for a lot of languages at this point, I've found a decent workflow to create good corpus rules, even if you know very little about a language. This takes a few steps:

#### Latin languages

1. Run `load <language> --raw`. This will create a data file in `static/language_data_raw` with letters unshifted and all control keys removed. It will however maintain everything else.

2. Once you have this file, you use it to get an idea on what keys are common, what keys should get a dedicated key on main layer, if you maybe want an accent key and for what keys, that kinda stuff.

(Accent key specific:)
3. If you do think you need an accent key, you need to figure out how it should work. Not all accents are equal: some accented characters might actually require their own key, like `é` in French, which occurs about 2.3% - much more common than some other letters of the 'regular' alphabet. For languages like Spanish which only have one instance of accents, using a format of `<accented letter> -> <accent key><letter>` is enough, but for French for example, which has a _lot_ of different accents, I still use a single accent key (denoted further with `*`).

The way I do this is have the most common set of accents act like this (for French this is `^`), but have the rest simulate a keypress inbetween the accent key and the actual letter. For example, `ú` becomes `* u`. This means that, with an accent key on the same hand as the keys that are supposed to be accented, you can pretend the whole sequence is alternated, where the 'space' is a key on homerow on the opposite hand. You go `*`->`<key on opposite hand>`->`<letter>`. This is fine because the rest is extremely low freq, so having three (comfortable!) presses to type them is super convenient.

You could theoretically use two accent keys, but I think this is only useful if there's two types of accents, and nothing else exists. Pretty often even if you have a lot of different accents, using a single one is fine: You can see this in Czech really well, where accents are all over the place, but there's only one or two for each letter, so you can use the same accent key and still make it work.

4. If you know some double keys are very common, you can consider using them as dedicated keys as well. This has to be done as corpus preprocessing with your own script, and cannot be set by these rules. I do have plans to change this however.

5. Once you have this figured out, this is _usually_ enough to create a full ruleset. You use `inherits = ["default"]`, add the non-default alphabet keys that you think deserve a dedicated key to `letters_to_lowercase`, and create a list `to_multiple` for keys that require the accent key.

#### Non-latin languages

The workflow for this is a bit different. You still use step 1 and 2 just fine, but the amount of characters might be wildly different, and you wouldn't want to use `inherits = ["default"]` because keeping latin characters is not what you want. Especially if there are a (lot) more characters in the alphabet, you will have to dive deeper into how typing is usually done within that lanugage if you're not sure, and how to optimize it. It might for example be a good idea to create rules with the language-specific IME/prediction in mind, if possible. Also keep in mind you might need to add script-specific punctuation if such a thing exists.
