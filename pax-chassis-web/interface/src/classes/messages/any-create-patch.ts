export class AnyCreatePatch {
    public idChain?: number[];
    public clippingIds?: number[][];
    public scrollerIds?: number[][];
    public zIndex?: number;

    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];

        this.clippingIds = jsonMessage["clipping_ids"];

        this.scrollerIds = jsonMessage["scroller_ids"];

        this.zIndex = jsonMessage["z_index"];
    }

    cleanUp(){
        this.idChain = [];
        this.clippingIds = [];
        this.scrollerIds = [];
        this.zIndex = -1;
    }
}

