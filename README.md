# zsh-history-to-fish

Yet another ZSH history to Fish history converter.

## Usage

```console
$ zsh-history-to-fish --help
Usage: zsh-history-to-fish <ZSH_HISTORY>

Arguments:
<ZSH_HISTORY>  The path to the zsh history file

Options:
-h, --help     Print help
-V, --version  Print version
```

i.e.

```console
$ zsh-history-to-fish ~/.zsh_history > ~/.local/share/fish/fish_history
```

## License

MIT. See [LICENSE](LICENSE).
