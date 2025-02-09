package cmd

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"sync"
	"time"

	"github.com/azusachino/rosemary/model"
	"github.com/spf13/cobra"
)

type Pair struct {
	Id  int
	Url string
}

var platformMapping = map[string]string{
	"yandere":      "https://yande.re",
	"konachan":     "https://konachan.com",
	"danbooru":     "https://danbooru.donmai.us",
	"gelbooru":     "https://gelbooru.com",
	"sankaku":      "https://chan.sankakucomplex.com",
	"safebooru":    "https://safebooru.org",
	"rule34":       "https://rule34.com",
	"tbib":         "https://tbib.org",
	"e621":         "https://e621.net",
	"hypnohub":     "https://hypnohub.net",
	"realbooru":    "https://realbooru.com",
	"rule34paheal": "https://rule34.paheal.net",
	"paheal":       "https://rule34.paheal.net",
}

func init() {
	rootCmd.AddCommand(gelbooruCmd)

	gelbooruCmd.Flags().StringP("platform", "p", "yandere", "download from which platform")

	gelbooruCmd.Flags().StringP("target", "t", "", "save images to target directory")

}

var gelbooruCmd = &cobra.Command{
	Use:   "gelbooru",
	Short: "Download images from gelbooru",
	Run: func(cmd *cobra.Command, args []string) {
		// download images from gelbooru
		platform, _ := cmd.Flags().GetString("platform")
		url, ok := platformMapping[platform]

		if !ok {
			log.Fatalf("Error: platform %s is not supported", platform)
		}
		httpClient := http.Client{
			Timeout: 5 * time.Minute,
		}
		resp, err := httpClient.Get(fmt.Sprintf("%s/post.json", url))
		if err != nil {
			log.Fatalf("Error: %v", err)
		}
		defer resp.Body.Close()
		// parse response to json array
		var posts []model.Gelbooru
		if err := json.NewDecoder(resp.Body).Decode(&posts); err != nil {
			log.Fatalf("Error: %v", err)
		}
		workloadCh := make(chan Pair, len(posts))
		// print posts
		for _, post := range posts {
			workloadCh <- Pair{Id: post.ID, Url: post.FileURL}
		}
		// concurrency control
		close(workloadCh)

		waitGroup := sync.WaitGroup{}
		waitGroup.Add(1)
		targetFolder, _ := cmd.Flags().GetString("target")
		if targetFolder == "" {
			targetFolder = "./"
		}
		// TODO: introduce a worker pool
		go func() {
			defer waitGroup.Done()
			for pair := range workloadCh {
				fmt.Printf("Downloading image %d from %s\n", pair.Id, pair.Url)
				resp, err := httpClient.Get(pair.Url)
				if err != nil {
					log.Fatalf("Error: %v", err)
				}
				f, err := os.Create(fmt.Sprintf("%s%c%d.jpg", targetFolder, os.PathSeparator, pair.Id))
				if err != nil {
					log.Fatalf("Error: %v", err)
				}
				if _, err := f.ReadFrom(resp.Body); err != nil {
					log.Fatalf("Error: %v", err)
				}
				resp.Body.Close()
				f.Close()
			}
		}()

		waitGroup.Wait()
		fmt.Println("Download finished")
	},
}
