**OS Module**

Provides operating system information and utilities.

**Functions:**

- `platform()` - Get the current operating system platform (linux, windows, macos, etc.)

**Fields:**

- `OS` - The current operating system platform (same as platform() function)

**Examples:**

```braise
let current_os = os.platform();
print("Running on: ${current_os}");

# Using the field syntax
print("OS: ${os.OS}");
```