# Package

version = "0.1.0"
author = "sudhanv09"
description = "A simple CLI tool to manage media"
license = "MIT"
srcDir = "src"
bin = @["pmc"]

# Dependencies

requires "nim >= 2.0"

requires "argparse >= 4.0.2"
requires "nanoid >= 0.2.0"

requires "checksums >= 0.2.1"
requires "db_connector >= 0.1.0"