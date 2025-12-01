## git-commit-gen

AI-assisted Git commit message generator for staged changes.

### Features
- Generates concise commits from staged diffs using your configured LLM.
- Supports default or custom prompt templates (`templates/*.md`).
- One-command workflow: generates, lets you edit via `git commit --edit -m`, or commits directly.
- Auto-creates config/templates on first run—no manual init.
- Uses an OpenAI-compatible Chat Completions endpoint (`POST /chat/completions` JSON). Works with OpenAI or any provider exposing the same schema.

### Installation
```bash
cargo install --path .
```

### Usage
- Generate and interactively choose action:
  ```bash
  git-commit gen
  ```
- Skip prompts and commit directly with editing:
  ```bash
  git-commit gen --yes
  ```
- Use a specific template:
  ```bash
  git-commit gen --template conventional
  ```
- Pick template from a menu (when `--template` not set):
  ```bash
  git-commit gen --pick-template
  ```
- Commit without opening editor:
  ```bash
  git-commit gen --yes --no-edit
  ```

### Configuration
Config lives in `~/.git-commit-gen/config.toml` and is created automatically on first run.  
Set your API info there or via the `Config` subcommand:
```bash
git-commit-gen config --api-key "<your key>" --base-url "https://api.example.com/v1" --model-id "gpt-4o"
```
Requirements for the endpoint:
- Must support OpenAI-style Chat Completions (`model`, `messages`, optional `temperature`, etc.).
- Auth is sent as `Authorization: Bearer <api-key>`.
- Endpoint path is `/chat/completions` appended to `base_url`.

### Templates
Two built-ins (`default`, `conventional`) live in `~/.git-commit-gen/templates`.  
Add more by dropping `<name>.md` files in that folder, then call with `--template <name>` or `--pick-template`.

### Prerequisites
- Git with staged changes (`git add ...`).
- Network access to your LLM endpoint; API key configured.
