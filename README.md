# Laravel Env Diff

A Rust tool to compare Laravel `.env` files with cached configuration and identify differences.

## Overview

This tool helps identify discrepancies between your Laravel `.env` file and the cached configuration file (`bootstrap/cache/config.php`). It's useful for debugging configuration issues where environment variables might not be properly reflected in the cached config.

## Features

- Compares `.env` values with cached configuration
- Case-insensitive null value comparison
- Identifies missing configuration sections
- Simple output showing only environment variables with differences

## Prerequisites

- Rust (for compilation)
- PHP (for reading cached config files)
- Laravel project with generated config cache

## Installation

1.  Clone or download this project
    ```bash
    git clone git@github.com:tommykw/laravel-env-diff.git
    cd laravel-env-diff
    ```
2. Build the binary:
   ```bash
   cargo build --release
   ```
3. Install the binary:
   ```bash
   cargo install --path .
   ```
   
   This will build and install the `laravel-env-diff` command to your Cargo bin directory.

## Usage

1. Navigate to your Laravel project root directory
2. Generate config cache if not already done:
   ```bash
   php artisan optimize
   ```
3. Run the tool:
   ```bash
   laravel-env-diff
   ```

## Output

The tool will output differences in this format:
```
=== Differences between .env and bootstrap/cache/config.php ===
[DIFF] DB_HOST
[DIFF] REDIS_PASSWORD
```

If no differences are found:
```
=== Differences between .env and bootstrap/cache/config.php ===
No differences found.
```

## How it works

1. **Parse .env file**: Extracts key-value pairs from your `.env` file
2. **Scan config files**: Reads `config/*.php` files to map `env()` calls to configuration sections
3. **Load cached config**: Uses PHP to load and serialize `bootstrap/cache/config.php`
4. **Compare values**: Checks if `.env` values are present in the cached configuration

## Note on Artisan Commands and Current Limitations

I wasn't aware that custom Artisan commands could be created, so I implemented this tool as a standalone Rust CLI.

Also, there are some cases that the current implementation does not yet cover, including:
- Environment variable expansion (e.g., `MAIL_FROM_NAME="${APP_NAME}"`)
- Values present in `$_ENV` but not in the `.env` file
- `.env.[environment]` files for environment-specific configuration
- Non-local file systems

## Error Handling

If `bootstrap/cache/config.php` doesn't exist, the tool will exit with an error message:
```
Config cache file not found: bootstrap/cache/config.php
Please run 'php artisan optimize' to generate config cache
```

## Testing

Run the test suite:
```bash
cargo test
```

## Contributing

Feel free to submit issues and pull requests to improve this tool.