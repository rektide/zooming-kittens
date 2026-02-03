# Project Structure

## Current Issues

The current structure has several concerns that should be better separated:

1. **Mixing CLI and business logic**: `main.rs` contains CLI parsing, event loop logic, and window management
2. **Registry too large**: `registry.rs` handles connections, commands, process mapping, and configuration - too many responsibilities
3. **Commands scattered**: Focus tracking logic in main.rs, font commands in separate module

## Proposed Structure

```
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
- **Purpose**: Parse command-line arguments
- **Contains**: clap-derived structs for all commands
- **Exports**: Parsed, typed command structures

### Commands Layer (`commands/`)
- **Purpose**: Execute user commands
- **Contains**: High-level command handlers
- **Dependencies**: Uses `kitty/` for operations, `config/` for settings
- **Examples**:
  - `fonts::handle_font_command()` - execute font commands
  - `focus::run_focus_tracker()` - run the event loop

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
   - Move relevant code from existing files

2. **Phase 2**: Extract interfaces
   - Define clear APIs between layers
   - Update imports across codebase

3. **Phase 3**: Test and refactor
   - Ensure all functionality still works
   - Add integration tests for layer boundaries

4. **Phase 4**: Clean up
   - Remove old files
   - Update documentation

## Benefits

1. **Testability**: Each layer can be unit tested independently
2. **Maintainability**: Changes to CLI don't affect kitty logic
3. **Reusability**: Kitty layer can be used by other tools
4. **Clarity**: Clear purpose for each module
5. **Growth**: Easy to add new commands or kitty operations
