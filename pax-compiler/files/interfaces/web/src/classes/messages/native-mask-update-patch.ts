export type NativeMaskEntry = {
    path: string;
    clips: string[];
    opacity?: number;
};

export class NativeMaskUpdatePatch {
    public id?: number;
    public sizeX?: number;
    public sizeY?: number;
    public entries: NativeMaskEntry[] = [];

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.entries = (jsonMessage["entries"] || []).map((entry: any) => ({
            path: entry["path"] || "",
            clips: entry["clips"] || [],
            opacity: entry["opacity"],
        }));
    }

    cleanUp() {
        this.id = undefined;
        this.sizeX = undefined;
        this.sizeY = undefined;
        this.entries = [];
    }
}
