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


export type History = {
    id: number;
    query: string;
    filters?: string;
    strategy: SearchStrategy;
    doc: string,
    peso_fts: number;
    peso_semantic: number;
    neighbors: number;
    fecha: string;
};

export type Field = {
    name: string;
    unique: boolean;
    vec_input: boolean;
};

export type Document = {
    name: string;
    fields: Field[];
};


export type ServerError =
    | InternalError
    | BadRequestError
    | ParsingErrorResponse;

export interface InternalError {
    err: string;
    date: string;
}

export interface BadRequestError {
    err: string;
    type: "invalid_fields";
    valid_fields: string[];
    invalid_fields: string[];
    date: string;
}

export type ParsingErrorResponse =
    | ParsingInvalidTokenError
    | ParsingGenericError;

export interface ParsingInvalidTokenError {
    err: string;
    type: "invalid_token";
    date: string;
}

export interface ParsingGenericError {
    err: string;
    type: "parsing_error";
    date: string;
}

