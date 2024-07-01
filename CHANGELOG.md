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
