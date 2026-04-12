export class ObjectPool<T> {
    private pool: T[] = [];
    private readonly factory: (args?: any) => T;
    private readonly cleanUp: (item: T) => void;
    private readonly reusable: boolean;

    constructor(
        factory: (args?: any) => T,
        cleanUp: (item: T) => void,
        reusable: boolean = true,
    ) {
        this.factory = factory;
        this.cleanUp = cleanUp;
        this.reusable = reusable;
    }

    get(args?: any): T {
        if (this.pool.length > 0) {
            return this.pool.pop() as T;
        }
        return this.factory(args);
    }

    put(item: T) {
        this.cleanUp(item);
        if (this.reusable) {
            this.pool.push(item);
        }
    }
}
