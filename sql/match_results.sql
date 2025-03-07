CREATE TABLE match (
    id TEXT NOT NULL PRIMARY KEY,
    player_1_id TEXT NOT NULL,
    player_2_id TEXT NOT NULL,
    games_to_win INTEGER,
    start_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE match_results (
    id TEXT NOT NULL PRIMARY KEY,
    player_1_score INTEGER,
    player_2_score INTEGER,
    end_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO match (
    id,
    player_1_id,
    player_2_id,
    games_to_win
) VALUES (
    "test",
    "p1",
    "p2",
    2
);

INSERT INTO match_results (
    id,
    player_1_score,
    player_2_score
) VALUES (
    "test",
    1,
    2
);

SELECT * FROM match as m
INNER JOIN match_results as mr ON m.id = mr.id;
