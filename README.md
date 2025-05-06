# Obscure HVP tool
A simple CLI tool to extract and create hvp archives from obscure game series **Support both PC and PS2 versions!**

## Current state:
- **Obscure 1:** fully finished and ready to use
- **Obscure 2:** planned

## How to use:
simply open a cmd or terminal and type `obscure-hvp --help` to see the full help message of the tool

a few examples:
- **extracting files from a hvp archive:**
  ```sh
  obscure-hvp extract "path-to-hvp"
  ```
- **creating a new hvp archive from extracted files:**
  ```sh
  obscure-hvp create "path-to-hvp" "path-to-extracted-files-folder"
  ```
  remember for faster import you can just leave the files that you want to change in `path-to-extracted-files-folder`.

### Notes
- for **quick hvp extraction** without the need of opening terminal just drag and drop a single hvp file inside the tool executable.
- for **quick hvp packing** without the need of opening terminal just drag and drop both the original hvp file and the extracted folder inside the tool executable.
