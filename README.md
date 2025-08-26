# rv
**rv** is a non-invasive AI code review for any type of workflow.

It works as a CLI tool easy to use and integrate in any kind of workflow and it allows to review the code that you are currently writing or code written by other developers on your project by extracting relevant information from your codebase and processing this data using LLMs.

## Features

- **Unix philosophy** <br> *rv* follows the Unix philosophy by providing one minimalstic tool (<1k LoC) that does one thing (code review) well.
- **Cheap and low-latency** <br> *rv* is optimized to use cheap and low-latency models in order to allow for reviews that takes less than 5 seconds and cost about $0.001 (on average, tested with gpt-4o-mini)
- **Deterministic** <br> *rv* uses deterministic sampling (LLM's temperature set to 0 and other parameters tweaked) in order to avoid anomalies in the output. <br> NOTE: Beacuse of issues like token tie-breaking or non-deterministic floating point operations on GPUs, results might not be fully deterministic; we plan on implementing seed support on supported providers in order to allow for fully deterministic prompting
- **Fully customizable** <br> *rv* is designed to give full freedom with its configuration file, allowing for different providers, LLMs and prompts
- **Semplicity of code** <br> *rv* is designed to be written using clean, understandable and safe (as in no `unsafe` instructions used) Rust code
- **Open source and non-monetized** <br> *rv* is released under the GPL license and we won't sell subscriptions, cloud credits or other form of monetized services to our end users

## How To Install

### From crates.io

Just run `cargo install rv-tool --version 1.0.0-rc3` in order to install the last version (the specified version is only needed on testing releases); then, follow the "From the source" guide from the third step.

### From the source

1. Clone the repository
2. Compile using `cargo build --release`
3. Run for the first time (just `rv`) in order to generate the configuration file
4. Edit the `~/.config/rv/config.toml` file setting up provider, model and API key
5. *rv* is now installed and ready! Run `rv` while you have staged edits (aka after `git add`) in order to get a code review of your current progress

NOTE: *rv* has been only tested on Linux; if possible try it on MacOS and Windows and open an issue with the results.

## How to setup APIs

We reccomend using [OpenRouter]() as it allows to use different models, connect to different APIs (such as Azure, Anthropic, Cloudflare, Google and Mistral), and access some free models.  
Here are the links for [creating an account](https://openrouter.ai/), [managing API keys](https://openrouter.ai/settings/keys), [connecting other provider](https://openrouter.ai/settings/integrations) and [viewing all free models](https://openrouter.ai/models?max_price=0).  
Once you have the API key, you can insert it in your configuration file (on Linux, `~/.config/rv/config.toml`).    

## Note about models

The current default model is `deepseek/deepseek-r1:free`, which provides great reasoning code reviews without having to pay.   
If your usage surpasses the limits of the free tier consider using `deepseek/deepseek-r1` and if you need a low-latency alternative, try `openai/gpt-4o-mini`.    
A good setup might be to have a low-latency configuration for most reviews and a reasoning configuration in case low-latency reviews are not enough for the current tasks; you can switch between different configurations using `-l`/`--llm`.

### Benchmarks

Tested with a simple commit with routine line changes (relevant as lots of diffs can induce certain models to allucinate).
All of the models in this table can do basic code review, but only the more advanced can do high-quality reports.
`x` symbols rappresent models that are not yet tested but are planned to be tested before the release of v1.0.0.

| Model    | Time Spent | Cost | Reasoning | Basic Test | Repetition Test | Coding Rating (LiveBench) |
|--|--|--|--|--|--|--|
|`openai/gpt-oss-20b`| x | x | ❌ | x | x | <58.80 |
|`openai/gpt-oss-120b`| x | x | ❌ | x | x | 58.80 |
|`openai/gpt-oss-20b`| x | x | ✅ | x | x | <58.80 |
|`openai/gpt-oss-120b`| x | x | ✅ | x | x | 58.80 |
|`google/gemini-2.5-flash-lite`| x | x | ❌ | x | x | <59.25 |
|`google/gemini-2.5-flash-lite`| x | x | ✅ | x | x | 59.25 |
|`google/gemini-2.5-flash`| x | x | ✅ | x | x | 63.53 |
|`openai/gpt-5-nano`| x | x | ✅ | x | x | 65.58 |
|`openai/gpt-5-nano`| x | x | ❌ | x | x | 65.58 |
|`openai/gpt-4o-mini`| 7s | 0,0003$ | ❌ | ✅ | ❌ | <69.29 |
|`openai/gpt-4o`| 11s | 0,006$ | ❌ | ✅ | ❌ | 69.29 |
|`openai/gpt-5-mini`| x | x | ❌ | x | x | 72.87 |
|`openai/gpt-5-mini`| x | x | ✅ | x | x | 72.87 |
|`deepseek/deepseek-r1`| 49s | 0,006$ | ✅ | ✅ | ✅ | 76.07 |
|`deepseek/deepseek-r1:free`| 50s | 0$ | ✅ | ✅ | ✅ | 76.07 |

## Future work

Milestones planned for the v1.0.0:
- full git support (not only staged edits, but also commits, branches and PRs)
- basic project context support (using README files, `.rv_context` and `.rv_guidelines`)
- custom prompt support
- raw mode support (selecting specific files or directory, skipping git integrations)
- support for alternative structure data formats for LLM input (allow usage of JSON or structured natural language instead of XML)

Milestones planned for the future:
- ollama support for local inference
- custom OpenAPI support
- *chat mode* for turning rv into a chatbot-like assistant
- *actions mode* for executing common git commands with one keystroke
- *fix mode* for producing and applying fixes from the code review using LLMs
- full project context support (indexed references to other code or text files, full project reviews for better security and architectural reports)
