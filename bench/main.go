package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"
)

func main() {
	data, err := os.ReadFile("bench/large.json")
	if err != nil {
		panic(err)
	}
	// mirror the kanso and serde harnesses: decode 150 times, report the mean
	const runs = 150
	var top int
	start := time.Now()
	for i := 0; i < runs; i++ {
		var v any
		if err := json.Unmarshal(data, &v); err != nil {
			panic(err)
		}
		top = len(v.([]any))
	}
	per := time.Since(start) / runs
	fmt.Printf("go decoded %d top-level values, mean over %d: %v\n", top, runs, per)
}
