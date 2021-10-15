// Package p contains an HTTP Cloud Function.
package p

import (
	"context"
	"encoding/json"
	"fmt"
	"html"
	"io"
	"log"
	"net/http"
	"os"
	"strings"

	"cloud.google.com/go/bigtable"
)

// fetch a single row by the row key
func ReadRow(w http.ResponseWriter, r *http.Request) {
	// Set CORS headers for the preflight request
	if r.Method == http.MethodOptions {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "POST")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		w.Header().Set("Access-Control-Max-Age", "3600")
		w.WriteHeader(http.StatusNoContent)
		return
	}
	// Set CORS headers for the main request.
	w.Header().Set("Access-Control-Allow-Origin", "*")

	var emitterChain, emitterAddress, sequence, rowKey string

	// allow GET requests with querystring params, or POST requests with json body.
	switch r.Method {
	case http.MethodGet:
		queryParams := r.URL.Query()
		emitterChain = queryParams.Get("emitterChain")
		emitterAddress = queryParams.Get("emitterAddress")
		sequence = queryParams.Get("sequence")

		readyCheck := queryParams.Get("readyCheck")
		if readyCheck != "" {
			// for running in devnet
			w.WriteHeader(http.StatusOK)
			fmt.Fprint(w, html.EscapeString("ready"))
			return
		}

		// check for empty values
		if emitterChain == "" || emitterAddress == "" || sequence == "" {
			fmt.Fprint(w, "query params ['emitterChain', 'emitterAddress', 'sequence'] cannot be empty")
			http.Error(w, http.StatusText(http.StatusBadRequest), http.StatusBadRequest)
			return
		}
	case http.MethodPost:
		// declare request body properties
		var d struct {
			EmitterChain   string `json:"emitterChain"`
			EmitterAddress string `json:"emitterAddress"`
			Sequence       string `json:"sequence"`
		}

		// deserialize request body
		if err := json.NewDecoder(r.Body).Decode(&d); err != nil {
			switch err {
			case io.EOF:
				fmt.Fprint(w, "request body required")
				return
			default:
				log.Printf("json.NewDecoder: %v", err)
				http.Error(w, http.StatusText(http.StatusBadRequest), http.StatusBadRequest)
				return
			}
		}

		// check for empty values
		if d.EmitterChain == "" || d.EmitterAddress == "" || d.Sequence == "" {
			fmt.Fprint(w, "body values ['emitterChain', 'emitterAddress', 'sequence'] cannot be empty")
			http.Error(w, http.StatusText(http.StatusBadRequest), http.StatusBadRequest)
			return
		}
		emitterChain = d.EmitterChain
		emitterAddress = d.EmitterAddress
		sequence = d.Sequence
	default:
		http.Error(w, "405 - Method Not Allowed", http.StatusMethodNotAllowed)
		log.Println("Method Not Allowed")
		return
	}

	// pad sequence to 16 characters
	if len(sequence) <= 15 {
		sequence = fmt.Sprintf("%016s", sequence)
	}
	// convert chain name to chainID
	if len(emitterChain) > 1 {
		chainNameMap := map[string]string{
			"solana":   "1",
			"ethereum": "2",
			"terra":    "3",
			"bsc":      "4",
		}
		lowercaseChain := strings.ToLower(emitterChain)
		if _, ok := chainNameMap[lowercaseChain]; ok {
			emitterChain = chainNameMap[lowercaseChain]
		}
	}
	rowKey = emitterChain + ":" + emitterAddress + ":" + sequence

	clientOnce.Do(func() {
		// Declare a separate err variable to avoid shadowing client.
		var err error
		project := os.Getenv("GCP_PROJECT")
		instance := os.Getenv("BIGTABLE_INSTANCE")
		client, err = bigtable.NewClient(context.Background(), project, instance)
		if err != nil {
			http.Error(w, "Error initializing client", http.StatusInternalServerError)
			log.Printf("bigtable.NewClient: %v", err)
			return
		}
	})

	tbl := client.Open("v2Events")
	row, err := tbl.ReadRow(r.Context(), rowKey)
	if err != nil {
		http.Error(w, "Error reading rows", http.StatusInternalServerError)
		log.Printf("tbl.ReadRows(): %v", err)
		return
	}
	if row == nil {
		http.NotFound(w, r)
		log.Printf("did not find row for key %v", rowKey)
		return
	}

	summary := makeSummary(row)
	jsonBytes, err := json.Marshal(summary)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte(err.Error()))
		log.Println(err.Error())
		return
	}
	w.WriteHeader(http.StatusOK)
	w.Write(jsonBytes)
}
