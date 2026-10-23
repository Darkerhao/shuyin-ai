-- 仅供 M0 技术验证；业务表在 M1 按单独迁移引入。
CREATE TABLE m0_probe (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    note TEXT NOT NULL CHECK (length(note) <= 2000),
    updated_at TEXT NOT NULL
);
