// Package rtblint provides Go bindings for the rtblint OpenRTB linter.
//
// rtblint validates OpenRTB 2.x and 3.0 bid requests and responses.
//
// Stub release — full CGo/WASM-backed implementation coming in 0.1.0.
//
// Basic usage (coming in 0.1.0):
//
//	result, err := rtblint.Validate(jsonString)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	if !result.Valid {
//	    for _, issue := range result.Issues {
//	        fmt.Printf("[%s] %s\n", issue.Severity, issue.Message)
//	    }
//	}
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
	Valid   bool
	Issues  []Issue
}

// Validate validates an OpenRTB JSON payload (bid request or response).
// Stub — returns ErrNotImplemented until 0.1.0.
func Validate(input string) (Result, error) {
	_ = input
	return Result{}, ErrNotImplemented
}

// ErrNotImplemented is returned by stub functions.
var ErrNotImplemented = errors.New("rtblint 0.0.1 is a stub — full implementation coming in 0.1.0")
