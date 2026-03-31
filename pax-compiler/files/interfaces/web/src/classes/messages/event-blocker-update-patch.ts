export class EventBlockerUpdatePatch {
    public id?: number;
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    public opacity?: number;
    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id = jsonMessage["id"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
            this.opacity = jsonMessage["opacity"];
        }
    }

    cleanUp(){
        this.id = undefined;
        this.sizeX = 0;
        this.sizeX = 0;
        this.transform = [];
        this.opacity = undefined;
    }
}
