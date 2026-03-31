export class FrameUpdatePatch {
    public id?: number;
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    public clipContent?: boolean;
    public clipPath?: string;
    public opacity?: number;

    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id = jsonMessage["id"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
            this.clipContent = jsonMessage["clip_content"];
            this.clipPath = jsonMessage["clip_path"];
            this.opacity = jsonMessage["opacity"];
        }
    }

    cleanUp(){
        this.id = undefined;
        this.sizeX = 0;
        this.sizeY = 0;
        this.transform = [];
        this.clipContent = undefined;
        this.clipPath = undefined;
        this.opacity = undefined;
    }
}
