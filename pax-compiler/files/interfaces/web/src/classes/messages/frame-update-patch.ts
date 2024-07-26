export class FrameUpdatePatch {
    public id?: number;
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    public clipContent?: boolean;

    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id = jsonMessage["id"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
            this.clipContent = jsonMessage["clip_content"];
        }
    }

    cleanUp(){
        this.id = undefined;
        this.sizeX = 0;
        this.sizeX = 0;
        this.transform = [];
    }
}
