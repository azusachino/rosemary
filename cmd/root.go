package cmd

import (
	"fmt"
	"log"

	"github.com/spf13/cobra"
)

const (
	APP_VERSION = "0.1.0"
)

var rootCmd = &cobra.Command{
	Use:   "rosemary",
	Short: "rosemary is a CLI tool for downloading images from booru sites",
}

func init() {
	// config args
	rootCmd.PersistentFlags().StringP("config", "c", "", "config file")

	// add sub commands
	rootCmd.AddCommand(versionCmd)
}

func Run() {
	if err := rootCmd.Execute(); err != nil {
		log.Fatalf("Error: %v", err)
	}
}

// sample sub commands
var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print the version number of rosemary CLI",
	Long:  `All software has versions. This is rosemary CLI's`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("rosemary CLI v%s\n", APP_VERSION)
	},
}
