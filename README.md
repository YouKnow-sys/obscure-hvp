# Obscure HVP tool
A simple CLI tool to extract and create hvp archives from obscure game series **Support both PC and PS2 versions!**

## Current state:
- **Obscure 1:** fully finished and ready to use
- **Obscure 2:** planned

## How to use:
simply open a cmd or terminal and type `obscure-hvp --help` to see the full help message of the tool

a few examples:
- **extracting files from a PS2 hvp file:**
  ```sh
  obscure-hvp -p ps2 extract "path-to-hvp"
  ```
- **creating a new PS2 archive from extracted files:**
  ```sh
  obscure-hvp -p ps2 create "path-to-hvp" "path-to-extracted-files-folder"
  ```
  remember for faster import you can just leave the files that you want to change in extracted-files-folder.
