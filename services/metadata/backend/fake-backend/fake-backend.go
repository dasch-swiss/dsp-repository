package main

import (
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
	"github.com/snabb/sitemap"
)

// Representation of a project
type Project struct {
	ID          string      `json:"id"`
	Name        string      `json:"name"`
	Description string      `json:"description"`
	Metadata    interface{} `json:"metadata"`
}

type spaHandler struct {
	staticPath string
	indexPath  string
}

// All projects that are being served
var projects []Project

// Full text search of a project.
// Returns a slice of Projects where each project matches the search query.
// Note: The query is a regex pattern and is matched against the JSON representation of the project.
func searchProjects(query string) []Project {
	var res []Project
	for _, project := range projects {
		content, _ := json.Marshal(project.Metadata)
		match, _ := regexp.Match("(?i)"+query, content)
		if match {
			res = append(res, project)
		}
	}
	return res
}

// Loads a project from a JSON file.
// Expects this file to be located in ./data/*.json
func loadProject(path string) Project {
	log.Printf("Loading: %v", path)
	// read json
	byteValue, err := ioutil.ReadFile(path)
	if err != nil {
		log.Fatal(err)
	}

	// unmarshal json
	jsonMap := make(map[string]interface{})
	err2 := json.Unmarshal(byteValue, &jsonMap)
	if err2 != nil {
		log.Fatal(err)
	}

	// get core information from json to build Project struct
	p, ok := jsonMap["project"].(map[string]interface{})
	if ok {
		id := p["shortcode"].(string)
		name := p["name"].(string)
		// default description: simply toString of description map
		description_map := p["description"].(map[string]interface{})
		description := fmt.Sprint(description_map)
		// if en, de or fr are in map, use specific string (in that order)
		if d, ok := description_map["en"]; ok {
			description = d.(string)
		} else if d, ok := description_map["de"]; ok {
			description = d.(string)
		} else if d, ok := description_map["fr"]; ok {
			description = d.(string)
		}
		return Project{
			ID:          id,
			Name:        name,
			Description: description,
			Metadata:    jsonMap,
		}
	} else {
		log.Fatal("Could not create project from JSON")
		return Project{}
	}
}

// Load Project Data
func loadProjectData() []Project {
	var res []Project

	pathPrefix := "./services/metadata/backend/fake-backend/data/"
	paths, _ := filepath.Glob(pathPrefix + "*.json")

	for _, path := range paths {
		file := filepath.Base(path)
		if !strings.HasPrefix(file, "_") {
			res = append(res, loadProject(path))
		}
	}

	return res
}

// Get projects
// Route: /projecs
func getProjects(w http.ResponseWriter, r *http.Request) {
	log.Printf("Request for: %v", r.URL)

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Expose-Headers", "X-Total-Count")

	// Request parameters
	query := r.URL.Query().Get("q")
	page, _ := strconv.Atoi(r.URL.Query().Get("_page"))
	limit, _ := strconv.Atoi(r.URL.Query().Get("_limit"))

	matches := make([]Project, len(projects))

	if query == "" {
		// no search query all projects are matches
		copy(matches, projects)
	} else {
		// reduce projects by search
		matches = searchProjects(query)
	}
	w.Header().Set("X-Total-Count", strconv.Itoa(len(matches)))
	// paginate
	if len(matches) > 1 && len(matches) > limit && page > 0 && limit > 0 {
		max := len(matches)
		start := (page - 1) * limit
		if start > max {
			start = max
		}
		end := page * limit
		if end > max {
			end = max
		}
		matches = matches[start:end]
	}
	// returns whatever remains
	json.NewEncoder(w).Encode(matches)
}

// Get a single project
// Route /projects/:id
func getProject(w http.ResponseWriter, r *http.Request) {
	log.Printf("Request for: %v", r.URL)

	w.Header().Set("Content-Type", "application/json")

	params := mux.Vars(r)
	for _, item := range projects {
		for item.ID == params["id"] {
			json.NewEncoder(w).Encode(item.Metadata)
			return
		}
	}
	json.NewEncoder(w).Encode(&Project{})
}

// getSitemap returns the sitemap.xml containing routes to all project pages.
// Route /sitemap.xml
func getSitemap(w http.ResponseWriter, r *http.Request) {
	log.Printf("Request for: %v", r.URL)

	sm := sitemap.New()
	sm.Add(&sitemap.URL{
		Loc:        "https://meta.dasch.swiss/",
		ChangeFreq: sitemap.Weekly,
	})

	for _, item := range projects {
		projectUrl := fmt.Sprintf("https://meta.dasch.swiss/projects/%s/", item.ID)
		sm.Add(&sitemap.URL{
			Loc:        projectUrl,
			ChangeFreq: sitemap.Weekly,
		})
	}
	sm.WriteTo(w)
}

// getRobotsFile returns the robots.txt file containing the reference to the sitemap
// Route /robots.txt
func getRobotsFile(w http.ResponseWriter, r *http.Request) {
	log.Printf("Request for: %v", r.URL)
	rf := "Sitemap: https://meta.dasch.swiss/sitemap.xml\nUser-agent: *\nDisallow:"
	io.WriteString(w, rf)
}

// servers the contnet version.txt file
// Route /version.txt
func getVersionFile(w http.ResponseWriter, r *http.Request) {
	log.Printf("Request for: %v", r.URL)
	vf, err := ioutil.ReadFile("./version.txt")
	if err != nil {
		fmt.Println("Error creating", "version.txt")
		fmt.Println(err)
		return
	}
	io.WriteString(w, string(vf))
}

// handle SPA to serve always from right place, no matter of route
func (h spaHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	log.Printf("SPA Handler: %v", r.URL)

	// get the absolute path to prevent directory traversal
	path, err := filepath.Abs(r.URL.Path)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// prepend the path with the path to the static directory
	path = filepath.Join(h.staticPath, path)

	// check whether a file exists at the given path
	_, err2 := os.Stat(path)
	if err2 == nil {
		// file exists -> serve file
		// log.Printf("Serving from File Server: %v", path)
		http.FileServer(http.Dir(h.staticPath)).ServeHTTP(w, r)
		return
	} else {
		// file does not exist, see where to go from here
		pattern := "/projects/?([0-9A-F]{4})?"
		match, _ := regexp.MatchString(pattern, path)
		if match {
			// file matches "/project/shortcode" pattern -> remove this section of the path
			re := regexp.MustCompile(pattern)
			s := re.ReplaceAllString(path, "/")
			_, err3 := os.Stat(s)
			if err3 == nil {
				// file exists after removing the section -> serve this file
				// log.Printf("Existis after changing: %v", s)
				http.ServeFile(w, r, s)
				return
			}
		}

		// file still not found, serve index.html
		http.ServeFile(w, r, filepath.Join(h.staticPath, h.indexPath))
	}
}

func main() {
	// CORS header
	ch := handlers.CORS(handlers.AllowedOrigins([]string{"*"}))

	// load Data
	projects = loadProjectData()
	log.Printf("Loaded Projects: %v", len(projects))

	// init Router
	router := mux.NewRouter()

	// set up routes
	router.HandleFunc("/api/v1/projects", getProjects).Methods("GET")
	router.HandleFunc("/api/v1/projects/{id}", getProject).Methods("GET")
	router.HandleFunc("/robots.txt", getRobotsFile).Methods("GET")
	router.HandleFunc("/sitemap.xml", getSitemap).Methods("GET")
	router.HandleFunc("/version.txt", getVersionFile).Methods("GET")

	// init SPA handler
	spa := spaHandler{
		staticPath: "public/metadata",
		indexPath:  "index.html",
	}

	// apply SPA handler
	router.PathPrefix("/").Handler(spa)

	// init server
	srv := &http.Server{
		Handler:      ch(router),
		Addr:         ":3000",
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
	}

	// run server
	log.Fatal(srv.ListenAndServe())
}
