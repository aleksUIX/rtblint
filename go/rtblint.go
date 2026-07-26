// Package rtblint provides Go bindings for the RTBlint OpenRTB linter.
//
// RTBlint validates OpenRTB 2.x bid requests and bid responses against
// versioned IAB spec snapshots.
//
// The Go binding is not implemented yet. The Rust CLI (crates.io: rtblint),
// the npm WASM package (npm: rtblint-core), and the MCP server (crates.io:
// rtblint-mcp) are the working surfaces today.
package rtblint

import "errors"

// Issue represents a single validation finding.
type Issue struct {
	ID       string
	Severity string
	Message  string
	Path     string
}

// Result is the output of a validation run.
type Result struct {
	Valid  bool
	Issues []Issue
}

// Validate validates an OpenRTB JSON payload (bid request or response).
// It returns ErrNotImplemented until the Go binding lands.
func Validate(input string) (Result, error) {
	_ = input
	return Result{}, ErrNotImplemented
}

// ErrNotImplemented is returned by stub functions.
var ErrNotImplemented = errors.New("the RTBlint Go binding is not implemented yet; use the Rust CLI or the npm package")
