# Project Structure

## Current Issues

The current structure has several concerns that should be better separated:

1. **Mixing CLI and business logic**: `main.rs` contains CLI parsing, event loop logic, and window management
2. **Registry too large**: `registry.rs` handles connections, commands, process mapping, and configuration - too many responsibilities
3. **CLI and Commands are tightly coupled but in separate layers**: CLI parsing (`cli/`) defines what commands exist, and command handlers (`commands/`) implement them. These layers are so closely related that separating them creates unnecessary indirection and makes the code harder to navigate.

## Updated Proposed Structure

```
src/
├── main.rs                      # Entry point only
├── cli/                          # CLI parsing AND command handlers together
│   ├── mod.rs
│   ├── args.rs                  # clap-derived Args structs
│   ├── fonts.rs                 # Font commands (inc/dec/set/list)
│   └── focus.rs                # Focus tracking event loop
├── kitty/                         # Kitty-specific logic
│   ├── mod.rs
│   ├── client.rs                 # Kitty connection wrapper
│   ├── registry.rs               # Connection pool and cache
│   └── process.rs               # Process discovery and PID mapping
├── config.rs                    # Configuration management
├── error.rs                     # Error types
└── util.rs                      # Utilities (systemd generation, etc.)
```

**Note**: CLI parsing and command handlers are merged into `cli/` since they are tightly coupled - CLI defines command structure and handlers implement them. Separating creates unnecessary indirection.
src/
├── main.rs                      # Entry point, minimal CLI parsing only
├── cli/                          # All CLI argument parsing
│   ├── mod.rs
│   └── args.rs                  # clap-derived Args structs
├── commands/                      # All command handlers
│   ├── mod.rs
│   ├── fonts.rs                  # Font size commands (inc/dec/set/list)
│   └── focus.rs                 # Focus tracking event loop
├── kitty/                         # Kitty-specific logic
│   ├── mod.rs
│   ├── client.rs                 # Kitty connection wrapper
│   ├── registry.rs               # Connection pool and cache
│   └── process.rs               # Process discovery and PID mapping
├── config.rs                    # Configuration management
├── error.rs                     # Error types
└── util.rs                     # Utilities (systemd generation, etc.)
```

## Separation of Concerns

### CLI Layer (`cli/`)
- **Purpose**: Parse command-line arguments AND execute commands
- **Contains**:
  - clap-derived Args structs for all commands
  - Command handlers (`fonts.rs`, `focus.rs`)
- **Exports**: Parsed command structures and handler functions
- **Why merged**: CLI parsing and command execution are tightly coupled - CLI defines what commands exist, handlers implement them. Keeping them together reduces indirection and makes navigation easier.
- **Examples**:
  - `fonts::handle_font_command()` - execute font commands
  - `focus::run_focus_tracker()` - run the event loop (future)

### Kitty Layer (`kitty/`)
- **Purpose**: Abstract kitty terminal emulator interactions
- **Contains**:
  - `client::KittyClient` - Wrapper around kitty-rc library
  - `registry::KittyRegistry` - Connection pool with caching
  - `process::find_kitty_master_pid()` - Process discovery
- **Responsibilities**:
  - Socket discovery
  - Connection management
  - Command execution
  - PID mapping (shell → kitty master)

### Config Layer (`config.rs`)
- **Purpose**: Load and validate configuration
- **Contains**: Structs for app settings
- **Sources**: Environment variables, config files, CLI args
- **Exports**: `Config` struct

### Error Layer (`error.rs`)
- **Purpose**: Centralized error handling
- **Contains**: Custom error types
- **Features**:
  - From trait implementations for all error types
  - Helpful error messages
  - Error conversion utilities

### Utility Layer (`util.rs`)
- **Purpose**: Helper functions that don't fit elsewhere
- **Contains**: One-off utilities
- **Examples**:
  - `print_systemd_service()` - systemd file generation
  - Environment variable helpers

## Migration Path

1. **Phase 1**: Create new directory structure
   - Create `cli/`, `kitty/`, `config/` directories
   - Merge `commands/` into `cli/`
   - Move relevant code from existing files

2. **Phase 2**: Extract interfaces
   - Define clear APIs between layers
   - Update imports across codebase

3. **Phase 3**: Test and refactor
   - Ensure all functionality still works
   - Add integration tests for layer boundaries

4. **Phase 4**: Clean up
   - Remove old `commands/` directory
   - Update documentation

## Benefits

1. **Testability**: Each layer can be unit tested independently
2. **Maintainability**: Changes to CLI don't affect kitty logic
3. **Reusability**: Kitty layer can be used by other tools
4. **Clarity**: Clear purpose for each module
5. **Growth**: Easy to add new commands or kitty operations
