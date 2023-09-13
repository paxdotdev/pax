export class FrameUpdatePatch {
    public id_chain?: number[];
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id_chain = jsonMessage["id_chain"];
            this.size_x = jsonMessage["size_x"];
            this.size_y = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
        }
    }

    cleanUp(){
        this.id_chain = [];
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
    }
}