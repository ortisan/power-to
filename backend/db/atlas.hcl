variable "database_url" {
  type        = string
  description = "PostgreSQL connection URL supplied by the runtime secret store"
  default     = getenv("DATABASE_URL")
}

// One environment definition serves local, CI, staging, and production.
// Select it with --env and provide DATABASE_URL outside the repository.
env {
  name = atlas.env
  url  = var.database_url

  migration {
    dir = "file://migrations"
  }
}
