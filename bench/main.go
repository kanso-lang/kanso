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
	start := time.Now()
	var v any
	if err := json.Unmarshal(data, &v); err != nil {
		panic(err)
	}
	fmt.Printf("go decoded %d top-level values in %v\n", len(v.([]any)), time.Since(start))
}
