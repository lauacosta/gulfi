export type TableContent = {
    msg: string;
    columns: string[];
    rows: string[][];
};

export type SearchResponse = {
    table: TableContent;
    pages: number;
    embedding?: string;
};
