export class FrameUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    public clipContent?: boolean;
    public clipPath?: string;
    public opacity?: number;
    public presentedBounds?: number[];
    public presentedClipBounds?: number[];

    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id = jsonMessage["id"];
            this.parentFrame = jsonMessage["parent_frame"];
            this.zIndex = jsonMessage["z_index"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
            this.clipContent = jsonMessage["clip_content"];
            this.clipPath = jsonMessage["clip_path"];
            this.opacity = jsonMessage["opacity"];
            this.presentedBounds = jsonMessage["presented_bounds"];
            this.presentedClipBounds = jsonMessage["presented_clip_bounds"];
        }
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.sizeX = 0;
        this.sizeY = 0;
        this.transform = [];
        this.clipContent = undefined;
        this.clipPath = undefined;
        this.opacity = undefined;
        this.presentedBounds = undefined;
        this.presentedClipBounds = undefined;
    }
}
