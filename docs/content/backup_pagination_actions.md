# Backup Pagination and Actions

The Logs & Backups page includes a paginated backup list to prevent overflow when many backups exist.

## Actions

- Restore: icon-only restore action. Current live files are backed up before restore, so restore is reversible.
- Delete: icon-only trash action. This permanently deletes the selected backup directory after admin confirmation.

## Safety

Backup delete is CSRF-protected, admin-only, and restricted to direct child directories under the configured backup directory. Every restore/delete action writes an audit event.
