export class ScrollerUpdatePatch {
    public idChain?: number[];
    public sizeX?: number;
    public sizeY?: number;
    public sizeInnerPaneX? : number;
    public sizeInnerPaneY? : number;
    public transform? : number[];
    public scrollX? : boolean;
    public scrollY? : boolean;
    public subtreeDepth?: number;

    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
        this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
        this.transform = jsonMessage["transform"];
        this.scrollX = jsonMessage["scroll_x"];
        this.scrollY = jsonMessage["scroll_y"];
        this.subtreeDepth = jsonMessage["subtree_depth"];
    }

    cleanUp(){
        this.idChain = [];
        this.sizeX = 0;
        this.sizeY = 0;
        this.sizeInnerPaneX = 0;
        this.sizeInnerPaneY = 0;
        this.transform = [];
        this.scrollX = false;
        this.scrollY = false;
        this.subtreeDepth = 0;
    }
}
