export class EventBlockerUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    public opacity?: number;
    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id = jsonMessage["id"];
            this.parentFrame = jsonMessage["parent_frame"];
            this.zIndex = jsonMessage["z_index"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
            this.opacity = jsonMessage["opacity"];
        }
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.sizeX = 0;
        this.sizeX = 0;
        this.transform = [];
        this.opacity = undefined;
    }
}
