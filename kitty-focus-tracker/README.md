# kitty-focus-tracker

> Monitor kitty terminal window focus events via niri IPC

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Install

### From source

```bash
cargo install --path kitty-focus-tracker
```

## Usage

### CLI

Run the tracker to monitor kitty terminal focus events:

```bash
kitty-focus-tracker
```

With verbose output:

```bash
kitty-focus-tracker --verbose
```

Track a different app ID:

```bash
kitty-focus-tracker --app-id "my-terminal"
```

Show help:

```bash
kitty-focus-tracker --help
```

### Output

The program prints focus events as JSON lines to stdout:

```json
{"event":"focus_gained","window_id":12345,"app_id":"kitty"}
```

```json
{"event":"focus_lost"}
```

## Contributing

PRs accepted.

Please open an issue for questions.

## License

SEE LICENSE IN LICENSE
