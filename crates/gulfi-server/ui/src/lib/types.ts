export type TableContent = {
    msg: string;
    columns: string[];

    rows: string[][];
};

export type searchStrategy = "Fts" | "Semantica" | "ReciprocalRankFusion";

export type favoritesResponse = {
    query: string;
    strategy: searchStrategy;
};

