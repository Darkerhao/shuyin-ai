ALTER TABLE editor_drafts RENAME TO editor_drafts_previous;
CREATE TABLE editor_drafts (
    owner_id TEXT NOT NULL,
    project_id TEXT REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('writing','cover','project','publishing')),
    based_on TEXT,
    content_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(owner_id, kind),
    CHECK(project_id IS NOT NULL OR (kind='project' AND owner_id='new'))
);
INSERT INTO editor_drafts(owner_id,project_id,kind,based_on,content_json,updated_at)
SELECT project_id,project_id,kind,based_on,content_json,updated_at FROM editor_drafts_previous;
DROP TABLE editor_drafts_previous;
CREATE INDEX editor_drafts_project ON editor_drafts(project_id);
