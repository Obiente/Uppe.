package statuspage

import (
	"fmt"
	"net/url"
	"regexp"
	"strings"
)

var slugPattern = regexp.MustCompile(`^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$`)
var colorPattern = regexp.MustCompile(`^#[0-9a-fA-F]{6}$`)

func validatePage(title, slug, description, logo, color string, ids []string) error {
	if strings.TrimSpace(title) == "" || len(title) > 200 {
		return fmt.Errorf("title must contain 1 to 200 characters")
	}
	if !slugPattern.MatchString(slug) {
		return fmt.Errorf("slug must contain 3 to 63 lowercase letters, numbers or hyphens")
	}
	if len(description) > 4000 {
		return fmt.Errorf("description is too long")
	}
	if color != "" && !colorPattern.MatchString(color) {
		return fmt.Errorf("color must be a six-digit hex color")
	}
	if logo != "" {
		u, err := url.Parse(logo)
		if err != nil || u.Scheme != "https" || u.Hostname() == "" || u.User != nil || len(logo) > 2048 {
			return fmt.Errorf("logo must be an HTTPS URL without credentials")
		}
	}
	if len(ids) > 100 {
		return fmt.Errorf("a status page can contain at most 100 monitors")
	}
	seen := map[string]bool{}
	for _, id := range ids {
		if len(id) != 36 || seen[id] {
			return fmt.Errorf("monitor identifiers must be unique UUIDs")
		}
		seen[id] = true
	}
	return nil
}
