export class YoutubeVideoUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public url?: string;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public opacity?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.zIndex = jsonMessage["z_index"];
        this.url = jsonMessage["url"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.opacity = jsonMessage["opacity"];
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.url = '';
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.opacity = undefined;
    }
}
