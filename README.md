# rv
**rv** is a non-invasive AI code review for any type of workflow.

It works as a CLI tool easy to use and integrate in any kind of workflow and it allows to review the code that you are currently writing or code written by other developers on your project by extracting relevant information from your codebase and processing this data using LLMs.

## Features

- **Unix philosophy** <br> *rv* follows the Unix philosophy by providing one minimalstic tool (<1k LoC) that does one thing (code review) well.
- **Cheap and low-latency** <br> *rv* is optimized to use cheap and low-latency models in order to allow for reviews that takes less than 5 seconds and cost about $0.001 (on average, tested with gpt-4o-mini)
- **Deterministic** <br> *rv* uses deterministic sampling (LLM's temperature set to 0 and other parameters tweaked) in order to avoid anomalies in the output.
- **Fully customizable** <br> *rv* is designed to give full freedom with its configuration file, allowing for different providers, LLMs and prompts
- **Semplicity of code** <br> *rv* is designed to be written using clean, understandable and safe (as in no `unsafe` instructions used) Rust code
- **Open source and non-monetized** <br> *rv* is released under the GPL license and we won't sell subscriptions, cloud credits or other form of monetized services to our end users

## How To Install

### From crates.io

Just run `cargo install rv-tool` in order to install the last version; then, follow the "From the source" guide from the third step.

### From the source

1. Clone the repository
2. Compile using `cargo build --release`
3. Run for the first time (just `rv`) in order to generate the configuration file
4. Edit the `~/.config/rv/config.toml` file setting up provider, model and API key
5. *rv* is now installed and ready! Run `rv` while you have staged edits (aka after `git add`) in order to get a code review of your current progress

NOTE: *rv* has been only tested on Linux; if possible try it on MacOS and Windows and open an issue with the results.

## Future work

Milestones planned for the v1.0.0:
- project context support (either using README or `.rv_context`)
- custom prompt support
- raw mode support (selecting specific files or directory, skipping git integrations)
- full git support (not only staged edits, but also commits, branches and PRs)

Milestones planned for the future:
- ollama support for local inference
- custom OpenAPI support
- *chat mode* for turning rv into a chatbot-like assistant
- *actions mode* for executing common git commands with one keystroke
