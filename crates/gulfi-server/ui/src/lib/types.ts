export type TableContent = {
    msg: string;
    columns: string[];

    rows: string[][];
};

export type SearchStrategy = "Fts" | "Semantic" | "ReciprocalRankFusion";

export type favoritesResponse = {
    query: string;
    strategy: SearchStrategy;
};


export type Historial = {
    id: number;
    query: string;
    strategy: SearchStrategy;
    sexo: "U" | "M" | "F";
    edad_min: number;
    edad_max: number;
    peso_fts: number;
    peso_semantic: number;
    neighbors: number;
    fecha: string;
};

