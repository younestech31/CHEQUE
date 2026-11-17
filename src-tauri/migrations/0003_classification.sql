-- Version 3: classification metadata and referential safeguards.
-- The operation_type column is added conditionally by the Rust migration runner.
CREATE INDEX IF NOT EXISTS idx_allocations_promoter_date ON allocations(promoter_id,allocation_date DESC,id DESC);
CREATE INDEX IF NOT EXISTS idx_allocations_client ON allocations(client_id,allocation_date DESC);
CREATE INDEX IF NOT EXISTS idx_allocations_collection ON allocations(collection_id,id);
CREATE INDEX IF NOT EXISTS idx_projects_promoter ON projects(promoter_id,name);
CREATE INDEX IF NOT EXISTS idx_clients_name ON clients(full_name);

CREATE TRIGGER trg_allocation_project_matches_promoter
BEFORE INSERT ON allocations WHEN NEW.project_id IS NOT NULL BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM projects WHERE id=NEW.project_id AND promoter_id=NEW.promoter_id) THEN RAISE(ABORT,'project does not belong to selected promoter') END;
END;
CREATE TRIGGER trg_allocation_client_valid
BEFORE INSERT ON allocations BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM clients WHERE id=NEW.client_id) THEN RAISE(ABORT,'unknown client') END;
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM promoters WHERE id=NEW.promoter_id AND active=1) THEN RAISE(ABORT,'promoter is inactive or unknown') END;
END;
CREATE TRIGGER trg_no_update_allocations BEFORE UPDATE ON allocations BEGIN SELECT RAISE(ABORT,'allocation history is immutable'); END;
CREATE TRIGGER trg_no_delete_allocations BEFORE DELETE ON allocations BEGIN SELECT RAISE(ABORT,'allocation history cannot be deleted'); END;
