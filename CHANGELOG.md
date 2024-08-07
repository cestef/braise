## v0.1.8 (2024-08-07)

### Fix

- include system env var

## v0.1.7 (2024-08-07)

### Feat

- trim output based on terminal size

## v0.1.6 (2024-07-02)

### Feat

- allow false for dotenv property
- add confirm to tasks
- add `--quiet` option for limiting output

### Fix

- trim multi-lines commands
- define intermidiate types for either<bool, u8> and either <bool, string>

## v0.1.5 (2024-07-02)

### Feat

- add `--init` command to generate a sample braise.toml file with the schema included

### Fix

- set runs-on as an array in the schema

## v0.1.4 (2024-07-01)

### Feat

- add `runs-on` property to specify the task's platform
- add support for default env values with env(VAR:default)
- add inline env var replacement with `env(VAR)`
- add dotenv support

### Fix

- join arguments with space
- correctly pass shell args

### Refactor

- move args and env replacements to util functions

## v0.1.3 (2024-07-01)

### Feat

- add `-d` to specify log level directly
- add debug and trace logs

### Fix

- get correct default shells

### Refactor

- remove config directory information
- move available tasks print to file.rs

## v0.1.2 (2024-07-01)

### Feat

- add default task
- add `quiet` and allow per-task overriding
- add `deps` property to tasks

### Fix

- hide output in quiet mode

### Refactor

- move task-related stuff to `task.rs`
- remove `paris` from deps

## v0.1.1 (2024-07-01)
