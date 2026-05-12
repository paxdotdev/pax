export class PhotoPickerUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public opacity?: number;
    public trigger?: number;
    public source?: string;
    public allowMultiple?: boolean;
    public accept?: string;
    public includeBytes?: boolean;
    public maxBytesPerPhoto?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.zIndex = jsonMessage["z_index"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.opacity = jsonMessage["opacity"];
        this.trigger = jsonMessage["trigger"];
        this.source = jsonMessage["source"];
        this.allowMultiple = jsonMessage["allow_multiple"];
        this.accept = jsonMessage["accept"];
        this.includeBytes = jsonMessage["include_bytes"];
        this.maxBytesPerPhoto = jsonMessage["max_bytes_per_photo"];
    }

    cleanUp() {
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.opacity = undefined;
        this.trigger = undefined;
        this.source = undefined;
        this.allowMultiple = undefined;
        this.accept = undefined;
        this.includeBytes = undefined;
        this.maxBytesPerPhoto = undefined;
    }
}
