package dev.pax;

import java.util.Random;

/** Runs on the host JDK; verifies the transform consumed by Android View properties. */
public final class AffineTest {
    public static void main(String[] args) {
        check(1,0,0,1);
        check(0,0,0,0);
        check(0,0,0,2);
        check(-1,0,0,1);
        check(1,0,3,1);
        check(1,2,2,4);
        Random random=new Random(1042);
        for(int i=0;i<10000;i++) check(random.nextGaussian(),random.nextGaussian(),random.nextGaussian(),random.nextGaussian());
        System.out.println("Affine: rotation, skew, reflection, singular, and 10000 random maps passed");
    }

    private static void check(double a,double b,double c,double d) {
        double[] parts=Affine.decompose(a,b,c,d);
        double u=Math.toRadians(parts[0]),v=Math.toRadians(parts[3]);
        double sx=parts[1],sy=parts[2];
        double[] actual={
            Math.cos(u)*sx*Math.cos(v)-Math.sin(u)*sy*Math.sin(v),
            Math.sin(u)*sx*Math.cos(v)+Math.cos(u)*sy*Math.sin(v),
            -Math.cos(u)*sx*Math.sin(v)-Math.sin(u)*sy*Math.cos(v),
            -Math.sin(u)*sx*Math.sin(v)+Math.cos(u)*sy*Math.cos(v)
        };
        double[] expected={a,b,c,d};
        for(int i=0;i<4;i++) if(!Double.isFinite(actual[i]) || Math.abs(actual[i]-expected[i])>1e-10)
            throw new AssertionError("Affine reconstruction differed at coefficient "+i);
    }
}
