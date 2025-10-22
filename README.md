<h1 align="center">Obscure HVP tool</h1>

<p align="center">
  <b>A simple CLI tool to extract and create hvp archives from obscure game series and Final Exam</b></br>
  <sub>Support PC, PS2, PSP and Wii versions</sub>
</p>

<div align="center">

[![Build Status](https://github.com/YouKnow-sys/obscure-hvp/actions/workflows/rust.yml/badge.svg)](https://github.com/YouKnow-sys/obscure-hvp/actions?workflow=Rust%20CI)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/YouKnow-sys/obscure-hvp/blob/master/LICENSE)

</div>

<p align="center">
  <a href="#supported-games">Supported Games</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

## Supported Games

| Game                     | Status        | Platforms                        |
|--------------------------|---------------|----------------------------------|
| Obscure 1                | Supported ✅  | PC, PS2, XBOX                    |
| Obscure 2                | Supported ✅  | PC, PS2, PSP, Wii                |
| Final Exam               | Supported ✅  | PC, (and probably PS3, Xbox 360) |

## Installation

You can download latest version of the program from **Release** page

## Usage
Run `obscure-hvp --help` to see all available options. Each subcommand has its own detailed help accessible via `obscure-hvp <command> --help`.

#### Extract Files from HVP Archive
```bash
# Extract to a folder named after the HVP file
obscure-hvp extract "game_data.hvp"

# Extract to a specific directory
obscure-hvp extract "game_data.hvp" "extracted_files"
```

#### Create New HVP Archive
```bash
# Create archive from extracted files
obscure-hvp create "new_archive.hvp" "extracted_files"

# Create without compression (faster, larger file)
obscure-hvp create "test_archive.hvp" "extracted_files" --skip-compression
```

#### Advanced Options
```bash
# Force specific game
obscure-hvp extract "unknown.hvp" --game obscure1

# Force update all files when creating (ignore modification detection)
obscure-hvp create "archive.hvp" "files" --update-all-files
```

## Notes
- when creating a new archive tool will check which file is modified and just read the modified files from disk, you can override this feature and force the tool to read all the files from disk using `--update-all-files` option.
- tool will autodetect the game from input hvp, but you can also set it manually using `--game` option.
- For **quick HVP extraction** without the need of opening a terminal, simply drag and drop a single HVP file onto the tool executable to extract it immediately.
- For **quick HVP packing** without the need of opening a terminal, drag and drop both the original HVP file and the extracted folder onto the tool executable to create a new archive automatically.

## Contributing

Contributions are welcome! Please open an issue or PR.

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for more details.

<p align="center">
  <i>Made with ❤️ for the Obscure community</i>
</p>
