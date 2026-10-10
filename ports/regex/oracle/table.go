// The oracle for `regex --table`: reads lines of PATTERN<TAB>SUBJECT and, for
// each, prints the line followed by a tab and either every match Go's regexp
// (which implements RE2's syntax and leftmost-first semantics) finds, or the
// error it reports. Offsets are counted in code points, not bytes, to agree
// with the kanso port, and "\n" in a subject stands for a newline.
//
//   go run oracle/table.go < tests/table_basic.tsv
//   go run oracle/table.go -L < tests/table_longest.tsv   (leftmost-longest)
package main

import (
	"bufio"
	"fmt"
	"os"
	"regexp"
	"strings"
	"unicode/utf8"
)

func main() {
	longest := len(os.Args) > 1 && os.Args[1] == "-L"
	in := bufio.NewScanner(os.Stdin)
	for in.Scan() {
		line := in.Text()
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.SplitN(line, "\t", 2)
		pattern := parts[0]
		subject := ""
		if len(parts) == 2 {
			subject = strings.ReplaceAll(parts[1], `\n`, "\n")
		}
		re, err := regexp.Compile(pattern)
		if err != nil {
			fmt.Printf("%s\t%s\n", line, err)
			continue
		}
		if longest {
			re.Longest()
		}
		all := re.FindAllStringSubmatchIndex(subject, -1)
		if len(all) == 0 {
			fmt.Printf("%s\tno match\n", line)
			continue
		}
		var out []string
		for _, m := range all {
			var slots []string
			for _, b := range m {
				if b < 0 {
					slots = append(slots, "-")
				} else {
					slots = append(slots, fmt.Sprint(utf8.RuneCountInString(subject[:b])))
				}
			}
			out = append(out, "("+strings.Join(slots, " ")+")")
		}
		fmt.Printf("%s\t%s\n", line, strings.Join(out, " "))
	}
}
