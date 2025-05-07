# paip

A command-line tool to interact with Large Language Models (LLMs) via stdin or files.

Input handling mimics POSIX cat in how it reads from files and stdin, but it collects all input before sending it to the LLM for processing, unlike cat which outputs immediately.

## Configuration

Before using paip, you need to configure your LLM API key and settings.

Run the following command to create a default configuration file:

```bash
paip --init-config
```

This will create a `config.yaml` file in the appropriate configuration directory for your system (e.g., `~/.config/paip/config.yaml`).
Edit this file to:
*   Specify the `provider` (e.g., `gemini`).
*   Add your LLM provider's API `key` under the corresponding provider section (e.g., under `gemini:`).
*   Configure other settings like the `timeout` (in milliseconds), model (e.g., `gemini-2.0-flash`), temperature, top_p, top_k, max_output_tokens, and thinking_budget under the provider section.

Currently, only the `gemini` provider is supported.

## Usage

```text
Usage: paip [OPTIONS] [FILES]...

Arguments:
  [FILES]...  Files to process. Reads from stdin if no files are provided. Use '-' to read from stdin within a list of files.

Options:
  -p, --prompt <PROMPT>  Use a predefined prompt from the configuration file.
      --init-config      Create a default configuration file if it doesn't exist.
  -v, --verbose          Enable verbose output for debugging.
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

Process text from stdin:
```bash
echo 'Summarize this text.' | paip
```

Process text from a file:
```bash
paip file.txt
```

Process text from multiple files:
```bash
paip file1.txt file2.txt
```

Process text from stdin and a file using a specific prompt:
```bash
echo 'Additional context.' | paip -p summarize file.txt -
```

Explain most recent git commit:
```bash
git show | paip
```

## License

GPL-2.0-only
