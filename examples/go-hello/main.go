package main

import (
	"fmt"
	"net/http"
)

func helloHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintln(w, "Hello, World!")
}

func main() {
	http.HandleFunc("/", helloHandler) // Handle requests to "/"
	http.ListenAndServe(":8080", nil)  // Start server on port 8080
}
