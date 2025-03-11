
export function getBgColor(strategy: string) {
    switch (strategy) {
        case "Fts":
            return "fts";
        case "ReciprocalRankFusion":
            return "reciprocal-rank-fusion";
        case "Semantic":
            return "semantic";
        default:
            return "";
    }
}
