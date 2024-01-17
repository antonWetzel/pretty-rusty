# Pretty-Rusty

Formatter for Rust

## Project State

- somewhat usable (based on my code and what I want)
	- features are added if I need them
	- defaults to single space between everything text

## Features / Design

- only modify whitespace
- no maximal line width
	- insert linebreaks based on source code
- format lists based on whitespace after open bracket
	- single line if no linebreak
	- multiline if linebreak

## Architecture

- use `rust-analyzer` abstract syntax tree
- iterate in depth-first order
- rules to select whitespace between two none-whitespace nodes
	- input
		- left node kind
		- whitespace between (may be none)
		- right node kind
		- parent node kind
		- scope
	- output
		- whitespace (may be none)
- calculate scope for complex rules (mostly lists)
- state with settings, indentation, ...

## Why not `rustfmt`

- because blank lines around top level items is unstable for 3 years
	- https://github.com/rust-lang/rustfmt/issues/3382
- formating fails with any invalid code in the file
