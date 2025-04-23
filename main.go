// SPDX-License-Identifier: GPL-3.0-or-later
package main

import (
	"fmt"
	"kanso-lang/repl"
	"os"
	"os/user"
)

func main() {
	currentUser, err := user.Current()
	if err != nil {
		fmt.Printf("Error getting current user: %v\n", err)
		return
	}

	fmt.Printf("Welcome to the Kanso REPL, %s!\n", currentUser.Username)
	repl.Start(os.Stdin)
}
