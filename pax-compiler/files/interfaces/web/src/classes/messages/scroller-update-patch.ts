export class ScrollerUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public sizeX?: number;
    public sizeY?: number;
    public clipContent?: boolean;
    public sizeInnerPaneX? : number;
    public sizeInnerPaneY? : number;
    public transform? : number[];
    public opacity? : number;
    public scrollX? : number;
    public scrollY? : number;
    public presentationScrollX?: number;
    public presentationScrollY?: number;
    public contentLayerId? : number;
    public presentedBounds? : number[];
    public presentedClipBounds? : number[];

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.zIndex = jsonMessage["z_index"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.clipContent = jsonMessage["clip_content"];
        this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
        this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
        this.transform = jsonMessage["transform"];
        this.opacity = jsonMessage["opacity"];
        this.scrollX = jsonMessage["scroll_x"];
        this.scrollY = jsonMessage["scroll_y"];
        this.presentationScrollX = jsonMessage["presentation_scroll_x"];
        this.presentationScrollY = jsonMessage["presentation_scroll_y"];
        this.contentLayerId = jsonMessage["content_layer_id"];
        this.presentedBounds = jsonMessage["presented_bounds"];
        this.presentedClipBounds = jsonMessage["presented_clip_bounds"];
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.sizeX = 0;
        this.sizeY = 0;
        this.clipContent = undefined;
        this.sizeInnerPaneX = 0;
        this.sizeInnerPaneY = 0;
        this.transform = [];
        this.opacity = undefined;
        this.scrollX = 0;
        this.scrollY = 0;
        this.presentationScrollX = 0;
        this.presentationScrollY = 0;
        this.contentLayerId = undefined;
        this.presentedBounds = undefined;
        this.presentedClipBounds = undefined;
    }
}
