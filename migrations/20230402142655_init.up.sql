-- Add up migration script here

CREATE TABLE "notes" (
    author VARCHAR(32) NOT NULL,
    date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    iv CHAR(24) NOT NULL,
    content VARCHAR(102400) NOT NULL,
    PRIMARY KEY (author, date)
);
