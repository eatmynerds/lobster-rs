# LOBSTER-RS

A [`lobster`](https://github.com/justchokingaround/lobster) rewrite in Rust. With a few improvements.

## Overview

- [Installation](#installation)
  - [NixOS](#nixos-flake)
  - [Mac](#mac)
  - [Windows](#windows)
- [Usage](#usage)
  - [`--clear-history`](#--clear-history-argument)
  - [`-d` / `--download`](#--d----download-path-argument)
  - [`-e` / `--edit`](#--e----edit-argument)
  - [`-i` / `--image-preview`](#--i----image-preview-argument)
  - [`-j` / `--json`](#--j----json-argument)
  - [`-l` / `--language`](#--l----language-language-argument)
  - [`--rofi`](#--rofi-argument)
  - [`-p` / `--provider`](#--p----provider-provider-argument)
  - [`-q` / `--quality`](#--q----quality-quality-argument)
  - [`--recent`](#--recent-tvmovie-argument)
  - [`-t` / `--trending`](#--t----trending-tvmovie-argument)
  - [`-c` / `--continue`](#c----continue-argument) 
  <!-- - [`--rpc`](#discord--discord-presence--rpc--presence-argument-todo) (TODO) -->
  <!-- - [`-s` / `--syncplay`](#s--syncplay-argument-todo) (TODO) -->
  - [`-u` / `--update`](#u----update-argument)
  - [`-V` / `--version`](#v----version-argument)
  - [`--debug`](#debug-argument)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [Uninstall](#uninstall)

## TODO:

#### Features:
- [ ] Implement `--rpc` / `--discord-presence` / `--presence` argument: Add support for Discord RPC presence.
- [ ] Implement `-s` / `--syncplay` argument: Enable syncplay functionality.

#### Platform Support:
- [ ] Add Android support.
- [ ] Add support for IINA (macOS media player).
- [ ] Add support for Termux (Linux-based terminal emulator for Android).

## Installation

### Prerequisites
Before you run the installer you'll need the following for it to work:
- [`jq`](https://jqlang.github.io/jq/)
- `unzip` - As most linux distributions don't come with it by default

#### Linux 

```sh
curl -sL https://github.com/eatmynerds/lobster-rs/raw/master/install -o install && \
chmod +x install && \
./install && \
sudo mv lobster-rs /usr/local/bin/lobster-rs && \
rm install && \
echo 'lobster-rs installed successfully! :) \nRun `lobster-rs --help` to get started.'
```

#### Nixos (Flake)

Add this to you flake.nix

```nix
inputs.lobster.url = "github:eatmynerds/lobster-rs";
```

Add this to you configuration.nix

```nix
environment.systemPackages = [
  inputs.lobster.packages.<architecture>.lobster
];
```

##### Or for run the script once use

```sh
nix run github:eatmynerds/lobster-rs
```

##### Nixos (Flake) update

When encoutering errors first run the nix flake update command in the cloned
project and second add new/missing [dependencies](#dependencies) to the
default.nix file. Use the
[nixos package search](https://search.nixos.org/packages) to find the correct
name.

```nix
nix flake update
```

#### Mac

```sh
curl -sL https://github.com/eatmynerds/lobster-rs/raw/master/install -o install && \
chmod +x install && \
./install && \
sudo mv lobster-rs "$(brew --prefix)"/bin/lobster-rs && \
rm install && \
echo 'lobster-rs installed successfully! :) \nRun `lobster-rs --help` to get started.'
```

#### Windows (Git Bash)

<details>
<summary>Windows installation instructions</summary>

- This guide covers how to install and use lobster with the windows terminal,
  you could also use a different terminal emulator, that supports fzf, like for
  example wezterm
- Note that the git bash terminal does _not_ have proper fzf support

1. Install scoop

Open a PowerShell terminal
https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows?view=powershell-7.2#msi
(version 5.1 or later) and run:

```ps
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
irm get.scoop.sh | iex
```

2. Install git,mpv and fzf

```ps
scoop bucket add extras
scoop install git mpv fzf
```

3. Install windows terminal (you don't need to have a microsoft account for
   that) https://learn.microsoft.com/en-us/windows/terminal/install

4. Install git bash (select the option to add it to the windows terminal during
   installation) https://git-scm.com/download/win

(The next steps are to be done in the windows terminal, in a bash shell)

5. Download the script file to the current directory

```sh
curl -sL https://github.com/eatmynerds/lobster-rs/raw/master/install -o install && \
chmod +x install && \
./install && \
sudo mv lobster-rs /usr/bin/lobster-rs && \
rm install && \
echo 'lobster-rs installed successfully! :) \nRun `lobster-rs --help` to get started.'

```

</details>

## Usage

```sh
lobster-rs --help
```

  Note:
    All arguments can be specified in the config file as well.
    If an argument is specified in both the config file and the command line, the command line argument will be used.

  Some example usages:
```sh
lobster-rs -i "a silent voice" --rofi
lobster-rs -l Spanish "fight club" -i -d
lobster-rs -l Spanish "blade runner" --json
```

<details>
<summary>Showcase</summary>

![image](https://github.com/justchokingaround/lobster/assets/44473782/5ed98fb9-008d-4068-a854-577245cfe1ee)

![image](https://github.com/justchokingaround/lobster/assets/44473782/cd59329e-a1c8-408a-be48-690db2d52642)

![image](https://github.com/justchokingaround/lobster/assets/44473782/fae5ea52-4dc4-41ee-b7a2-cbb2476f5819)

</details>

### `--clear-history` argument

This argument allows you to delete the history file

```sh
lobster-rs --clear-history
```

### `-d` / `--download` `<path>` argument

This option lets you use lobster as you normally would, with the exception that
instead of playing the video in your player of choice, it will instead download
the video. If no path is specified when passing this argument, then it will
download to the current working directory, as an example, it would look like
this:

```sh
lobster-rs -d . "rick and morty"
```

or

```sh
lobster-rs "rick and morty" -d
```

If you want to specify a path to which you would like to download the video, you
can do so by passing an additional parameter to the `-d` or `--download`
argument, for instance: using a full path:

```sh
lobster-rs -d "/home/nerds/tv_shows/rick_and_morty/" "rick and morty"
```

or using a relative path:

```sh
lobster-rs -d "../rick_and_morty/" "rick and morty"
```

### `-e` / `--edit` argument

By passing this argument you can edit the config file using an editor of your
choice. By default it will use the editor defined in the `~/.config/lobster-rs/config.toml`
file, but if you don't have one defined, it will use the `$EDITOR` environment
variable (if it's not set, it will default to `vim`).

### `-i` / `--image-preview` argument

By passing this argument you can see image previews when selecting an entry.

For `rofi` it will work out of the box, if you have icons enabled in your
default configuration.

Example using my custom rofi configuration (to customize how your rofi image
preview looks, please check the [configuration](#configuration) section)

<details>
<summary>Showcase</summary>

![image](https://github.com/justchokingaround/lobster/assets/44473782/a8850f00-9491-4f86-939d-2f63bcb36e96)

</details>

For `fzf` you will need to install
[chafa](https://github.com/hpjansson/chafa/)

<details>
<summary>Showcase</summary>

![image](https://github.com/justchokingaround/lobster/assets/44473782/8d8057d8-4d85-4f0e-b6c0-3b7dd5dce557)

</details>

<summary>Installation instructions for chafa</summary>

On Arch Linux you can install it using your aur helper of choice with:

```sh
paru -S chafa
```

### `-j` / `--json` argument

By passing this argument, you can output the json for the currently selected
media to stdout, with the decrypted video link.

### `-l` / `--language` `<language>` argument

By passing this argument, you can specify your preferred language for the
subtitles of a video. 
Example use case:

```sh
lobster-rs "seven" -l Spanish
```

NOTE: The default language is `english`.

### `--rofi` argument

By passing this argument, you can use rofi instead of fzf to interact with the
lobster script.

This is the recommended way to use lobster, and is a core philosophy of this
script. My use case is that I have a keybind in my WM configuration that calls
lobster, that way I can watch Movies and TV Shows without ever even opening the
terminal.

Here is an example of that looks like (without image preview):

<details>
<summary>Showcase</summary>

![image](https://github.com/justchokingaround/lobster/assets/44473782/d1243c17-0ef1-44b3-99a8-f2c4a4ab5da9)

</details>

### `-p` / `--provider` `<provider>` argument

By passing this argument, you can specify a preferred provider. The script
currently supports the following providers: `Upcloud`, `Vidcloud`. 
Example use case:

```sh
lobster-rs -p Vidcloud "shawshank redemption"
```

### `-q` / `--quality` `<quality>` argument

By passing this argument, you can specify a preferred quality for the video (if
those are present in the source). If it is not provided as an argument the quality
will default to the highest available one.

Example use case:

```sh
lobster-rs -q 720 "the godfather"
```

### `--recent` `<tv|movie>` argument

By passing this argument, you can see watch most recently released movies and TV
shows. You can specify if you want to see movies or TV shows by passing the `tv`
or `movie` parameter. 

Example use case:

```sh
lobster-rs --recent tv
```

### `-t` / `--trending` `<tv|movie>` argument

By passing this argument, you can see the most trending movies and TV shows.

Example use case:

```sh
lobster-rs -t movie
```

### `-c` / `--continue` argument

This feature is disabled by default because it relies on history, to enable it,
you need to change the following line in your configuration file:

```sh
history=true
```

In a similar fashion to how saving your position when you watch videos on
YouTube or Netflix works, lobster has history support and saves the last minute
you watched for a Movie or TV Show episode. To use this feature, simply watch a
Movie or an Episode from a TV Show, and after you quit mpv the history will be
automatically updated. The next time you want to resume from the last position
watched, you can just run

```sh
lobster-rs --continue
```

which will prompt you to chose which of the saved Movies/TV Shows you'd like to
resume from. Upon the completion of a movie or an episode, the corresponding
entry is either deleted (in case of a movie, or the last episode of a show), or
it is updated to the next available episode (if it's the last episode of a
season, it will update to the first episode of the next season).

### `-u` / `--update` argument

By passing this argument, you can update the script to the latest version.

Example use case:

```sh
lobster-rs -u
```

### `-V` / `--version` argument

By passing this argument, you can see the current version of the script. This is
useful if you want to check if you have the latest version installed.

### `--debug` argument

By passing this argument, you can see the debug output of the script. 

## Configuration

Please refer to the
[wiki](https://github.com/justchokingaround/lobster/wiki/Configuration) for
information on how to configure the script using the config file.

## Dependencies

- fzf
- mpv
- rofi (external menu)
- vlc (optional)
- chafa (optional)
- ffmpeg (optional)

### In case you don't have fzf installed, you can install it like this:

```sh
git clone --depth 1 https://github.com/junegunn/fzf.git ~/.fzf
~/.fzf/install
```


