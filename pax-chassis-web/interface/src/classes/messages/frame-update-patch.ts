export class FrameUpdatePatch {
    public idChain?: number[];
    public sizeX?: number;
    public sizeY?: number;
    public transform?: number[];
    fromPatch(jsonMessage: any) {
        if(jsonMessage != null) {
            this.idChain = jsonMessage["id_chain"];
            this.sizeX = jsonMessage["size_x"];
            this.sizeY = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
        }
    }

    cleanUp(){
        this.idChain = [];
        this.sizeX = 0;
        this.sizeX = 0;
        this.transform = [];
    }
}
