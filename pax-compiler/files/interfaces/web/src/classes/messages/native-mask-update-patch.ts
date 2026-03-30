export type NativeMaskEntry = {
    path: string;
    clips: string[];
};

export class NativeMaskUpdatePatch {
    public id?: number;
    public entries: NativeMaskEntry[] = [];

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.entries = (jsonMessage["entries"] || []).map((entry: any) => ({
            path: entry["path"] || "",
            clips: entry["clips"] || [],
        }));
    }

    cleanUp() {
        this.id = undefined;
        this.entries = [];
    }
}
